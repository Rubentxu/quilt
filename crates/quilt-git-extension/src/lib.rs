//! Git extension plugin for quilt-mcp
//!
//! Provides git tools (status, log, diff) for AI agents using the git2 crate.
//! All operations are read-only - no push, pull, or commit operations.
//!
//! NOTE: This crate is not available on wasm32 targets since git2 does not support wasm.

use serde::Deserialize;
use serde_json::Value;
#[cfg(not(target_arch = "wasm32"))]
use std::path::PathBuf;
use tracing::instrument;

pub use quilt_mcp::plugin::{Plugin, PluginContext, PluginError, PluginManifest};
pub use quilt_mcp::tools::Tool;

#[cfg(not(target_arch = "wasm32"))]
/// Git plugin for quilt-mcp
///
/// Provides read-only git operations including status, log, and diff.
#[derive(Debug, Clone)]
pub struct GitPlugin {
    repo_path: PathBuf,
}

#[cfg(not(target_arch = "wasm32"))]
impl GitPlugin {
    /// Create a new GitPlugin for the given repository path.
    #[instrument]
    pub fn new(repo_path: PathBuf) -> Self {
        Self { repo_path }
    }

    /// Open the git repository at the configured path.
    fn open_repo(&self) -> Result<git2::Repository, PluginError> {
        git2::Repository::open(&self.repo_path).map_err(|e| {
            PluginError::InitFailed(format!(
                "Failed to open repository at {:?}: {}",
                self.repo_path, e
            ))
        })
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl Plugin for GitPlugin {
    fn name(&self) -> &str {
        "git"
    }

    fn version(&self) -> &str {
        "0.1.0"
    }

    fn tools(&self) -> Vec<Tool> {
        vec![
            Tool {
                name: "git::status".to_string(),
                description: "Returns current branch, clean/dirty status, staged files".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {},
                    "required": []
                }),
            },
            Tool {
                name: "git::log".to_string(),
                description: "Returns recent commits with hash, message, author, date".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "max_count": {
                            "type": "integer",
                            "description": "Maximum number of commits to return",
                            "default": 10
                        },
                        "author": {
                            "type": "string",
                            "description": "Filter by author name or email"
                        }
                    },
                    "required": []
                }),
            },
            Tool {
                name: "git::diff".to_string(),
                description: "Returns uncommitted changes (no args = all changes)".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "staged": {
                            "type": "boolean",
                            "description": "Show staged changes instead of unstaged",
                            "default": false
                        },
                        "path": {
                            "type": "string",
                            "description": "Filter diff to specific path"
                        }
                    },
                    "required": []
                }),
            },
            Tool {
                name: "git::blame".to_string(),
                description: "Returns blame info for a file (lines with commit, author, summary)"
                    .to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "path": {
                            "type": "string",
                            "description": "Path to the file to blame (required)",
                        }
                    },
                    "required": ["path"]
                }),
            },
        ]
    }

    fn execute_tool(&self, name: &str, args: Value) -> Result<Value, PluginError> {
        match name {
            "git::status" => self.git_status(args),
            "git::log" => self.git_log(args),
            "git::diff" => self.git_diff(args),
            "git::blame" => self.git_blame(args),
            _ => Err(PluginError::NotFound(name.to_string())),
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl GitPlugin {
    /// git::status implementation - returns branch, clean/dirty status, staged files.
    #[instrument(skip(self))]
    fn git_status(&self, _args: Value) -> Result<Value, PluginError> {
        let repo = self.open_repo()?;

        // Get current branch
        let branch = repo
            .head()
            .ok()
            .and_then(|h| h.shorthand().map(String::from))
            .unwrap_or_else(|| "HEAD detached".to_string());

        // Check if repo is clean
        let statuses = repo
            .statuses(None)
            .map_err(|e| PluginError::ExecutionFailed(format!("Failed to get status: {}", e)))?;

        let is_clean = statuses.is_empty();

        // Categorize files
        let mut staged = Vec::new();
        let mut modified = Vec::new();
        let mut untracked = Vec::new();

        for entry in statuses.iter() {
            let path = entry.path().unwrap_or("").to_string();
            let status = entry.status();

            if status.is_index_new()
                || status.is_index_modified()
                || status.is_index_deleted()
                || status.is_index_renamed()
            {
                staged.push(path.clone());
            }
            if status.is_wt_modified() || status.is_wt_deleted() || status.is_wt_renamed() {
                modified.push(path.clone());
            }
            if status.is_wt_new() {
                untracked.push(path);
            }
        }

        Ok(serde_json::json!({
            "branch": branch,
            "clean": is_clean,
            "staged": staged,
            "modified": modified,
            "untracked": untracked
        }))
    }

    /// git::log implementation - returns recent commits.
    #[instrument(skip(self))]
    fn git_log(&self, args: Value) -> Result<Value, PluginError> {
        #[derive(Deserialize)]
        struct LogArgs {
            max_count: Option<usize>,
            author: Option<String>,
        }

        let args: LogArgs = serde_json::from_value(args).unwrap_or(LogArgs {
            max_count: Some(10),
            author: None,
        });

        let repo = self.open_repo()?;
        let mut revwalk = repo.revwalk().map_err(|e| {
            PluginError::ExecutionFailed(format!("Failed to create revwalk: {}", e))
        })?;

        // Handle case where repo has no commits yet (unborn HEAD)
        if revwalk.push_head().is_err() {
            // No HEAD or no commits - return empty log
            return Ok(serde_json::json!([]));
        }

        let max_count = args.max_count.unwrap_or(10);
        let author_filter = args.author;

        let mut commits = Vec::new();

        for (i, oid) in revwalk.enumerate() {
            if i >= max_count {
                break;
            }

            let oid =
                oid.map_err(|e| PluginError::ExecutionFailed(format!("Revwalk error: {}", e)))?;
            let commit = repo.find_commit(oid).map_err(|e| {
                PluginError::ExecutionFailed(format!("Failed to find commit: {}", e))
            })?;

            // Apply author filter if provided
            if let Some(ref author_filter) = author_filter {
                let author = commit.author();
                let author_str = format!(
                    "{} <{}>",
                    author.name().unwrap_or(""),
                    author.email().unwrap_or("")
                );
                if !author_str
                    .to_lowercase()
                    .contains(&author_filter.to_lowercase())
                {
                    continue;
                }
            }

            let timestamp = commit.time();
            let datetime = chrono::DateTime::from_timestamp(commit.time().seconds(), 0)
                .map(|dt| dt.to_rfc3339())
                .unwrap_or_default();

            commits.push(serde_json::json!({
                "hash": commit.id().to_string(),
                "short_hash": commit.id().to_string()[..7].to_string(),
                "message": commit.message().unwrap_or("").trim().to_string(),
                "author": {
                    "name": commit.author().name().unwrap_or(""),
                    "email": commit.author().email().unwrap_or("")
                },
                "date": datetime,
                "timestamp": timestamp.seconds()
            }));
        }

        Ok(serde_json::json!(commits))
    }

    /// git::diff implementation - returns unified diff string.
    #[instrument(skip(self))]
    fn git_diff(&self, args: Value) -> Result<Value, PluginError> {
        #[derive(Deserialize)]
        struct DiffArgs {
            staged: Option<bool>,
            path: Option<String>,
        }

        let args: DiffArgs = serde_json::from_value(args).unwrap_or(DiffArgs {
            staged: Some(false),
            path: None,
        });

        let repo = self.open_repo()?;
        let path_filter = args.path.clone();

        let diff = if args.staged.unwrap_or(false) {
            // Diff staged changes (index vs HEAD) - compare HEAD to index
            // Handle case where repo has no commits yet
            let head = match repo.head() {
                Ok(h) => h,
                Err(_) => {
                    // No HEAD yet - return empty diff
                    return Ok(serde_json::json!({
                        "staged": true,
                        "path": args.path,
                        "diff": ""
                    }));
                }
            };
            let head_oid = head
                .peel_to_commit()
                .map_err(|e| {
                    PluginError::ExecutionFailed(format!("Failed to peel to commit: {}", e))
                })?
                .id();
            let head_tree = repo
                .find_commit(head_oid)
                .map_err(|e| PluginError::ExecutionFailed(format!("Failed to find commit: {}", e)))?
                .tree()
                .map_err(|e| PluginError::ExecutionFailed(format!("Failed to get tree: {}", e)))?;

            repo.diff_tree_to_index(Some(&head_tree), None, None)
                .map_err(|e| {
                    PluginError::ExecutionFailed(format!("Failed to diff staged: {}", e))
                })?
        } else {
            // Diff working directory (workdir vs index)
            // diff_index_to_workdir compares index to working directory
            repo.diff_index_to_workdir(None, None).map_err(|e| {
                PluginError::ExecutionFailed(format!("Failed to diff workdir: {}", e))
            })?
        };

        let mut diff_text = String::new();
        diff.print(
            git2::DiffFormat::Patch,
            |delta: git2::DiffDelta, _hunk: Option<git2::DiffHunk>, line: git2::DiffLine| {
                // Apply path filter if provided
                if let Some(ref path) = path_filter {
                    if let Some(file_path) = delta.new_file().path().or(delta.old_file().path()) {
                        if file_path.to_string_lossy() != *path {
                            return true; // Skip this line
                        }
                    }
                }

                let prefix = match line.origin() {
                    '+' => "+",
                    '-' => "-",
                    ' ' => " ",
                    _ => "",
                };
                diff_text.push_str(prefix);
                diff_text.push_str(std::str::from_utf8(line.content()).unwrap_or(""));
                diff_text.push('\n');
                true
            },
        )
        .map_err(|e| PluginError::ExecutionFailed(format!("Failed to print diff: {}", e)))?;

        Ok(serde_json::json!({
            "staged": args.staged.unwrap_or(false),
            "path": args.path,
            "diff": diff_text
        }))
    }

    /// git::blame implementation - returns blame info per line.
    #[instrument(skip(self))]
    fn git_blame(&self, args: Value) -> Result<Value, PluginError> {
        #[derive(Deserialize)]
        struct BlameArgs {
            path: String,
        }

        let args: BlameArgs = serde_json::from_value(args)
            .map_err(|e| PluginError::ExecutionFailed(format!("Invalid blame args: {}", e)))?;

        let repo = self.open_repo()?;

        // Get the blame for the file
        let blame = repo
            .blame_file(std::path::Path::new(&args.path), None)
            .map_err(|e| {
                PluginError::ExecutionFailed(format!("Failed to blame file '{}': {}", args.path, e))
            })?;

        let mut lines = Vec::new();

        for hunk in blame.iter() {
            let final_commit_id = hunk.final_commit_id();
            let final_start_line = hunk.final_start_line();
            let final_lines_in_hunk = hunk.lines_in_hunk();

            // Get commit info if available
            let (commit_hash, author_name, author_email, summary) =
                if let Ok(commit) = repo.find_commit(final_commit_id) {
                    let author = commit.author();
                    (
                        final_commit_id.to_string(),
                        author.name().unwrap_or("").to_string(),
                        author.email().unwrap_or("").to_string(),
                        commit.summary().unwrap_or("").to_string(),
                    )
                } else {
                    (
                        final_commit_id.to_string(),
                        "Unknown".to_string(),
                        "".to_string(),
                        "Unknown".to_string(),
                    )
                };

            // Each hunk may span multiple lines - add an entry for each
            for i in 0..final_lines_in_hunk {
                let line_number = final_start_line + i;
                lines.push(serde_json::json!({
                    "line_number": line_number,
                    "commit": commit_hash,
                    "author": {
                        "name": author_name,
                        "email": author_email
                    },
                    "summary": summary
                }));
            }
        }

        Ok(serde_json::json!({
            "path": args.path,
            "lines": lines
        }))
    }
}

#[cfg(all(not(target_arch = "wasm32"), test))]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn create_test_repo() -> (TempDir, git2::Repository) {
        let temp = TempDir::new().unwrap();
        let repo = git2::Repository::init(temp.path()).unwrap();
        (temp, repo)
    }

    #[test]
    fn test_git_plugin_new() {
        let plugin = GitPlugin::new(PathBuf::from("/tmp"));
        assert_eq!(plugin.name(), "git");
        assert_eq!(plugin.version(), "0.1.0");
    }

    #[test]
    fn test_git_plugin_tools() {
        let plugin = GitPlugin::new(PathBuf::from("/tmp"));
        let tools = plugin.tools();
        assert_eq!(tools.len(), 4);
        assert_eq!(tools[0].name, "git::status");
        assert_eq!(tools[1].name, "git::log");
        assert_eq!(tools[2].name, "git::diff");
        assert_eq!(tools[3].name, "git::blame");
    }

    #[test]
    fn test_git_status_clean_repo() {
        let (temp, _repo) = create_test_repo();
        let plugin = GitPlugin::new(temp.path().to_path_buf());
        let result = plugin.git_status(serde_json::json!({})).unwrap();
        // Fresh repo has "HEAD detached" since there are no commits
        assert!(result["clean"].as_bool().unwrap_or(true));
        assert!(result["staged"].as_array().unwrap().is_empty());
        assert!(result["modified"].as_array().unwrap().is_empty());
        assert!(result["untracked"].as_array().unwrap().is_empty());
    }

    #[test]
    fn test_git_log_empty_repo() {
        let (temp, _repo) = create_test_repo();
        let plugin = GitPlugin::new(temp.path().to_path_buf());
        // Empty repo has no commits - should return empty array or error gracefully
        let result = plugin.git_log(serde_json::json!({"max_count": 10}));
        // Either empty array or error is acceptable for empty repo
        if result.is_ok() {
            assert!(result.unwrap().as_array().unwrap().is_empty());
        }
    }

    #[test]
    fn test_git_diff_empty_repo() {
        let (temp, _repo) = create_test_repo();
        let plugin = GitPlugin::new(temp.path().to_path_buf());
        let result = plugin.git_diff(serde_json::json!({})).unwrap();
        assert_eq!(result["staged"], false);
        assert!(result["diff"].as_str().unwrap_or("").is_empty());
    }

    #[test]
    fn test_git_diff_staged_empty_repo() {
        let (temp, _repo) = create_test_repo();
        let plugin = GitPlugin::new(temp.path().to_path_buf());
        // Staged diff on empty repo will fail since there's no HEAD
        let result = plugin.git_diff(serde_json::json!({"staged": true}));
        // This is expected to fail since there's no HEAD commit
        assert!(result.is_err() || result.unwrap()["diff"].as_str().unwrap_or("").is_empty());
    }

    #[test]
    fn test_git_blame_missing_file() {
        let (temp, _repo) = create_test_repo();
        let plugin = GitPlugin::new(temp.path().to_path_buf());
        // Blame on non-existent file should fail
        let result = plugin.git_blame(serde_json::json!({"path": "nonexistent.txt"}));
        assert!(result.is_err());
    }

    #[test]
    fn test_git_blame_on_committed_file() {
        use git2::Signature;

        let (temp, repo) = create_test_repo();

        // Create a file and commit it
        let file_path = temp.path().join("test.txt");
        std::fs::write(&file_path, "line 1\nline 2\nline 3\n").unwrap();

        // Stage and commit the file
        let mut index = repo.index().unwrap();
        index.add_path(std::path::Path::new("test.txt")).unwrap();
        index.write().unwrap();

        let tree_id = index.write_tree().unwrap();
        let tree = repo.find_tree(tree_id).unwrap();

        let sig = Signature::now("Test Author", "test@example.com").unwrap();
        repo.commit(Some("HEAD"), &sig, &sig, "Initial commit", &tree, &[])
            .unwrap();

        let plugin = GitPlugin::new(temp.path().to_path_buf());
        let result = plugin
            .git_blame(serde_json::json!({"path": "test.txt"}))
            .unwrap();

        assert_eq!(result["path"], "test.txt");
        let lines = result["lines"].as_array().unwrap();
        assert_eq!(lines.len(), 3);

        // All lines should have the same commit (from initial commit) - git2 uses SHA-1 hashes (40 hex chars)
        let first_commit = lines[0]["commit"].as_str().unwrap();
        assert_eq!(
            first_commit.len(),
            40,
            "commit hash should be 40 hex chars (SHA-1)"
        );
        assert!(
            first_commit.chars().all(|c| c.is_ascii_hexdigit()),
            "commit hash should be hex"
        );
        assert_eq!(lines[0]["author"]["name"], "Test Author");
    }
}
