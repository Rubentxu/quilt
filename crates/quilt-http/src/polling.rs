//! File Polling Service
//!
//! Polls the filesystem for changes to markdown files and broadcasts events
//! to SSE clients. Runs as a background task that periodically checks
//! for file modifications, creations, and deletions.
//!
//! # Design
//!
//! - Polls every N seconds (configurable via POLL_INTERVAL_SECS env var)
//! - Tracks file metadata (mtime) to detect changes
//! - Deduplicates events using a hash of path + mtime
//! - Broadcasts events via a tokio broadcast channel

use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;
use tracing::{debug, error, info, warn};

/// File change event
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum FileChangeEvent {
    /// A file was created
    Created { path: String },
    /// A file was modified
    Modified { path: String },
    /// A file was deleted
    Deleted { path: String },
}

impl FileChangeEvent {
    /// Get the path for this event
    pub fn path(&self) -> &str {
        match self {
            FileChangeEvent::Created { path } => path,
            FileChangeEvent::Modified { path } => path,
            FileChangeEvent::Deleted { path } => path,
        }
    }
}

/// Metadata for tracking file changes
#[derive(Debug, Clone)]
struct FileMetadata {
    /// Last modified time (in seconds since epoch)
    mtime: u64,
    /// File size
    size: u64,
}

/// Deduplication key for file events
#[derive(Debug, Clone, Eq)]
struct EventKey {
    path: PathBuf,
    mtime: u64,
}

impl PartialEq for EventKey {
    fn eq(&self, other: &Self) -> bool {
        self.path == other.path && self.mtime == other.mtime
    }
}

impl Hash for EventKey {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.path.hash(state);
        self.mtime.hash(state);
    }
}

/// PollingService configuration
#[derive(Debug, Clone)]
pub struct PollingConfig {
    /// Poll interval in seconds
    pub poll_interval_secs: u64,
    /// Whether to include hidden files
    pub include_hidden: bool,
    /// File extensions to watch
    pub extensions: Vec<String>,
}

impl Default for PollingConfig {
    fn default() -> Self {
        Self {
            poll_interval_secs: std::env::var("POLL_INTERVAL_SECS")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(5),
            include_hidden: false,
            extensions: vec![
                "md".to_string(),
                "markdown".to_string(),
                "org".to_string(),
            ],
        }
    }
}

/// PollingService - monitors filesystem for file changes
pub struct PollingService {
    config: PollingConfig,
    vault_path: PathBuf,
    /// Sender for broadcasting events to SSE clients
    event_tx: broadcast::Sender<FileChangeEvent>,
    /// Last known state of files (path -> metadata)
    last_state: std::sync::Mutex<HashMap<PathBuf, FileMetadata>>,
    /// Set of paths we've seen (to detect deletions)
    known_paths: std::sync::Mutex<HashSet<PathBuf>>,
}

use std::collections::HashSet;

/// Create a new PollingService
pub fn create(
    vault_path: PathBuf,
    event_tx: broadcast::Sender<FileChangeEvent>,
    config: PollingConfig,
) -> Arc<PollingService> {
    Arc::new(PollingService {
        config,
        vault_path,
        event_tx,
        last_state: std::sync::Mutex::new(HashMap::new()),
        known_paths: std::sync::Mutex::new(HashSet::new()),
    })
}

impl PollingService {
    /// Start the polling loop
    pub async fn run(self: Arc<Self>) {
        info!(
            "Starting PollingService with interval {}s",
            self.config.poll_interval_secs
        );

        // Initial scan
        if let Err(e) = self.scan_files().await {
            error!("Initial file scan failed: {}", e);
        }

        // Main polling loop
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(
            self.config.poll_interval_secs,
        ));

        loop {
            interval.tick().await;

            if let Err(e) = self.check_for_changes().await {
                error!("Error checking for file changes: {}", e);
            }
        }
    }

    /// Perform initial scan of all files
    async fn scan_files(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        debug!("Scanning files in vault: {:?}", self.vault_path);

        let mut last_state = self.last_state.lock().unwrap();
        let mut known_paths = self.known_paths.lock().unwrap();

        self.walk_dir(&self.vault_path, &mut |path| {
            if let Ok(metadata) = std::fs::metadata(&path) {
                if metadata.is_file() {
                    let mtime = metadata
                        .modified()
                        .ok()
                        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                        .map(|d| d.as_secs())
                        .unwrap_or(0);

                    let size = metadata.len();

                    let meta = FileMetadata { mtime, size };
                    last_state.insert(path.clone(), meta);
                    known_paths.insert(path);
                }
            }
        })?;

        debug!("Initial scan found {} files", known_paths.len());
        Ok(())
    }

    /// Check for file changes since last scan
    async fn check_for_changes(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let mut last_state = self.last_state.lock().unwrap();
        let mut known_paths = self.known_paths.lock().unwrap();

        let mut current_paths: HashSet<PathBuf> = HashSet::new();
        let mut new_paths: HashSet<PathBuf> = HashSet::new();
        let mut modified_paths: HashSet<PathBuf> = HashSet::new();

        // Walk current directory
        self.walk_dir(&self.vault_path, &mut |path| {
            current_paths.insert(path.clone());

            if let Ok(metadata) = std::fs::metadata(&path) {
                if metadata.is_file() {
                    let mtime = metadata
                        .modified()
                        .ok()
                        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                        .map(|d| d.as_secs())
                        .unwrap_or(0);

                    let size = metadata.len();

                    match last_state.get(&path) {
                        Some(prev_meta) => {
                            // Check if modified
                            if prev_meta.mtime != mtime || prev_meta.size != size {
                                modified_paths.insert(path.clone());
                            }
                        }
                        None => {
                            // New file
                            new_paths.insert(path.clone());
                        }
                    }

                    // Update state
                    last_state.insert(
                        path.clone(),
                        FileMetadata { mtime, size },
                    );
                }
            }
        })?;

        // Find deleted files - those in known_paths but not in current_paths
        let deleted_paths: Vec<PathBuf> = known_paths
            .difference(&current_paths)
            .cloned()
            .collect();

        // Broadcast events
        let tx = self.event_tx.clone();

        // Deleted files
        for path in &deleted_paths {
            let path_str = path.to_string_lossy().to_string();
            debug!("File deleted: {}", path_str);
            let event = FileChangeEvent::Deleted { path: path_str };
            let _ = tx.send(event);
        }

        // New files
        for path in &new_paths {
            let path_str = path.to_string_lossy().to_string();
            debug!("File created: {}", path_str);
            let event = FileChangeEvent::Created { path: path_str };
            let _ = tx.send(event);
        }

        // Modified files
        for path in &modified_paths {
            let path_str = path.to_string_lossy().to_string();
            debug!("File modified: {}", path_str);
            let event = FileChangeEvent::Modified { path: path_str };
            let _ = tx.send(event);
        }

        // Update known paths
        *known_paths = current_paths;

        // Remove deleted files from last_state
        for path in &deleted_paths {
            last_state.remove(path);
        }

        Ok(())
    }

    /// Walk directory and call callback for each file path
    fn walk_dir<F>(&self, dir: &Path, cb: &mut F) -> Result<(), Box<dyn std::error::Error + Send + Sync>>
    where
        F: FnMut(PathBuf),
    {
        if !dir.is_dir() {
            return Ok(());
        }

        for entry in std::fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();

            // Skip hidden files if configured
            if !self.config.include_hidden {
                if let Some(name) = path.file_name() {
                    if name.to_string_lossy().starts_with('.') {
                        continue;
                    }
                }
            }

            if path.is_dir() {
                self.walk_dir(&path, cb)?;
            } else if path.is_file() {
                // Check extension
                if let Some(ext) = path.extension() {
                    let ext_str = ext.to_string_lossy().to_lowercase();
                    if self.config.extensions.contains(&ext_str) {
                        cb(path);
                    }
                }
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn create_test_vault() -> (TempDir, PathBuf) {
        let temp_dir = TempDir::new().unwrap();
        let vault_path = temp_dir.path().to_path_buf();

        // Create test files
        fs::write(vault_path.join("test.md"), "# Test").unwrap();
        fs::write(vault_path.join("another.org"), "* Org file").unwrap();

        (temp_dir, vault_path)
    }

    #[tokio::test]
    async fn test_file_metadata_tracking() {
        let (_temp_dir, vault_path) = create_test_vault();
        let (tx, _rx) = broadcast::channel(100);
        let service = create(vault_path.clone(), tx, PollingConfig::default());

        // Initial scan
        service.scan_files().await.unwrap();

        let last_state = service.last_state.lock().unwrap();
        assert_eq!(last_state.len(), 2);

        let known_paths = service.known_paths.lock().unwrap();
        assert_eq!(known_paths.len(), 2);
    }

    #[tokio::test]
    async fn test_detect_new_file() {
        let (_temp_dir, vault_path) = create_test_vault();
        let (tx, mut rx) = broadcast::channel(100);
        let service = create(vault_path.clone(), tx, PollingConfig::default());

        // Initial scan
        service.scan_files().await.unwrap();

        // Create new file
        fs::write(vault_path.join("new-file.md"), "# New").unwrap();

        // Check for changes
        service.check_for_changes().await.unwrap();

        // Should receive a created event
        let event = rx.recv().await.unwrap();
        assert!(matches!(event, FileChangeEvent::Created { path } if path.contains("new-file.md")));
    }

    #[test]
    fn test_file_change_event_serialization() {
        let event = FileChangeEvent::Created {
            path: "/pages/test.md".to_string(),
        };
        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("\"type\":\"Created\""));
        assert!(json.contains("\"/pages/test.md\""));

        let event = FileChangeEvent::Modified {
            path: "/pages/test.md".to_string(),
        };
        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("\"type\":\"Modified\""));
    }
}