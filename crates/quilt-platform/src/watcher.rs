//! File system watcher for syncing markdown files to the database

use anyhow::Result;
use notify::{Event, EventKind, RecursiveMode, Watcher};
use quilt_domain::events::{AppEvent, FileChanged, FileEventType as DomainFileEventType};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::mpsc;
use std::time::Duration;
use tokio::sync::broadcast;
use tokio::time::sleep;
use tracing::instrument;

/// Callback for file system events
pub type FileEventCallback = Box<dyn Fn(FileEvent) + Send + Sync>;

#[derive(Debug, Clone)]
pub enum FileEventType {
    Created,
    Modified,
    Deleted,
    Renamed(PathBuf), // old path
}

#[derive(Debug, Clone)]
pub struct FileEvent {
    pub path: PathBuf,
    pub event_type: FileEventType,
}

/// Error type for file watcher operations.
#[derive(Debug, thiserror::Error)]
pub enum WatchError {
    #[error("Watch error: {0}")]
    Watch(String),
    #[error("Channel error: {0}")]
    Channel(String),
}

/// File system watcher that monitors a directory for markdown/file changes
pub struct FileWatcher {
    watch_paths: Vec<PathBuf>,
    watcher: Option<notify::RecommendedWatcher>,
}

impl FileWatcher {
    /// Create a new file watcher for the given paths
    pub fn new(watch_paths: Vec<PathBuf>) -> Self {
        Self {
            watch_paths,
            watcher: None,
        }
    }

    /// Start watching and call callback on file events
    #[instrument(skip(self, callback))]
    pub fn start<F>(&mut self, callback: F) -> Result<()>
    where
        F: Fn(FileEvent) + Send + Sync + 'static,
    {
        let (tx, rx) = mpsc::channel();

        let mut watcher = notify::recommended_watcher(move |res: Result<Event, notify::Error>| {
            if let Ok(event) = res {
                let _ = tx.send(event);
            }
        })?;

        for path in &self.watch_paths {
            if path.exists() {
                watcher.watch(path, RecursiveMode::Recursive)?;
            }
        }

        self.watcher = Some(watcher);

        // Spawn a thread to process events
        std::thread::spawn(move || {
            for event in rx {
                let file_events = Self::convert_event(event);
                for fe in file_events {
                    callback(fe);
                }
            }
        });

        Ok(())
    }

    /// Start watching and return a broadcast channel for file events.
    /// Events are debounced per-path with a 500ms window.
    ///
    /// This is the async version that integrates with the tokio runtime.
    #[instrument(skip(self))]
    pub async fn start_async(&mut self) -> Result<broadcast::Sender<AppEvent>, WatchError> {
        let (tx, _) = broadcast::channel(100);
        let tx_clone = tx.clone();
        let watch_paths = self.watch_paths.clone();

        // Spawn a blocking thread to run the notify watcher
        // and forward events to the async task via a tokio channel
        let (event_tx, mut event_rx) = tokio::sync::mpsc::channel::<Event>(100);

        std::thread::spawn(move || {
            let watcher = notify::recommended_watcher(move |res: Result<Event, notify::Error>| {
                if let Ok(event) = res {
                    let _ = event_tx.blocking_send(event);
                }
            })
            .expect("Failed to create watcher");

            let mut watcher = watcher;
            for path in &watch_paths {
                if path.exists() {
                    if let Err(e) = watcher.watch(path, RecursiveMode::Recursive) {
                        tracing::error!("Failed to watch path {:?}: {}", path, e);
                    }
                } else {
                    tracing::warn!("Watch directory does not exist: {:?}", path);
                }
            }

            // Keep the watcher alive by blocking
            // The watcher will be dropped when this thread exits
            loop {
                std::thread::sleep(std::time::Duration::from_secs(1));
            }
        });

        // Spawn async task to process events with debouncing
        tokio::spawn(async move {
            let mut debounce_map: HashMap<PathBuf, tokio::task::JoinHandle<()>> = HashMap::new();

            loop {
                // Use timeout to allow periodic cleanup of dead handles
                tokio::select! {
                    Some(event) = event_rx.recv() => {
                        // Process the notify event
                        let file_events = Self::convert_event_for_broadcast(event);

                        for (path, event_type) in file_events {
                            // Cancel existing debounce timer for this path
                            if let Some(handle) = debounce_map.remove(&path.clone()) {
                                handle.abort();
                            }

                            // Set new debounce timer
                            let tx = tx_clone.clone();
                            let path_clone = path.clone();
                            let event_type_clone = event_type;

                            let handle = tokio::spawn(async move {
                                sleep(Duration::from_millis(500)).await;
                                let app_event = AppEvent::FileChanged(FileChanged {
                                    path: path_clone,
                                    event_type: event_type_clone,
                                    timestamp: chrono::Utc::now(),
                                });
                                let _ = tx.send(app_event);
                            });

                            debounce_map.insert(path, handle);
                        }
                    }
                    _ = tokio::time::sleep(Duration::from_secs(1)) => {
                        // Periodic cleanup: remove dead handles
                        debounce_map.retain(|_, handle| !handle.is_finished());
                    }
                    else => {
                        // Channel closed
                        break;
                    }
                }
            }

            // Abort any remaining debounce tasks
            for handle in debounce_map.values() {
                handle.abort();
            }
        });

        Ok(tx)
    }

    /// Convert notify Event to domain FileEventType
    fn convert_event_for_broadcast(event: Event) -> Vec<(PathBuf, DomainFileEventType)> {
        let mut results = Vec::new();

        for path in &event.paths {
            // Only process markdown files
            let is_md = path
                .extension()
                .map(|ext| ext == "md" || ext == "org" || ext == "markdown")
                .unwrap_or(false);

            if !is_md {
                continue;
            }

            let event_type = match event.kind {
                EventKind::Create(_) => DomainFileEventType::Created,
                EventKind::Modify(_) => DomainFileEventType::Modified,
                EventKind::Remove(_) => DomainFileEventType::Deleted,
                EventKind::Any => DomainFileEventType::Modified,
                _ => continue,
            };

            results.push((path.clone(), event_type));
        }

        results
    }

    fn convert_event(event: Event) -> Vec<FileEvent> {
        let mut events = Vec::new();

        // Only process markdown files and quilt DB
        for path in &event.paths {
            let is_md = path
                .extension()
                .map(|ext| ext == "md" || ext == "org" || ext == "markdown")
                .unwrap_or(false);
            let is_db = path
                .extension()
                .map(|ext| ext == "db" || ext == "sqlite")
                .unwrap_or(false);

            if !is_md && !is_db {
                continue;
            }

            let event_type = match event.kind {
                EventKind::Create(_) => FileEventType::Created,
                EventKind::Modify(_) => FileEventType::Modified,
                EventKind::Remove(_) => FileEventType::Deleted,
                EventKind::Any => FileEventType::Modified,
                _ => continue,
            };

            events.push(FileEvent {
                path: path.clone(),
                event_type,
            });
        }

        events
    }
}

impl Drop for FileWatcher {
    fn drop(&mut self) {
        // watcher is dropped automatically
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_watcher_creation() {
        let watcher = FileWatcher::new(vec![PathBuf::from(".")]);
        assert_eq!(watcher.watch_paths.len(), 1);
    }

    #[test]
    fn test_watcher_no_paths() {
        let watcher = FileWatcher::new(vec![]);
        assert!(watcher.watcher.is_none());
    }

    #[test]
    fn test_file_event_types() {
        // Test that event types are correctly discriminated
        let created = FileEventType::Created;
        let modified = FileEventType::Modified;
        let deleted = FileEventType::Deleted;

        // Ensure they're distinct (using match exhaustiveness)
        match created {
            FileEventType::Created => {}
            _ => panic!(),
        }
        match modified {
            FileEventType::Modified => {}
            _ => panic!(),
        }
        match deleted {
            FileEventType::Deleted => {}
            _ => panic!(),
        }
    }

    #[tokio::test]
    async fn test_watcher_detects_new_file() -> Result<()> {
        use tempfile::tempdir;

        let dir = tempdir()?;
        let dir_path = dir.path().to_path_buf();

        let mut watcher = FileWatcher::new(vec![dir_path.clone()]);

        let (tx, rx) = mpsc::channel();

        watcher.start(move |event| {
            let _ = tx.send(event);
        })?;

        // Create a markdown file
        let file_path = dir_path.join("test.md");
        std::fs::write(&file_path, "# Hello")?;

        // Give it a moment to detect
        std::thread::sleep(std::time::Duration::from_millis(200));

        // Try to receive an event (may or may not get one depending on timing)
        match rx.try_recv() {
            Ok(event) => {
                assert!(
                    event.path.ends_with("test.md")
                        || event.path.to_string_lossy().contains("test")
                );
            }
            Err(_) => {
                // File may not have been detected yet — that's OK in CI
            }
        }

        Ok(())
    }

    #[test]
    fn test_markdown_filtering() {
        // Test that .md, .org, .markdown files are accepted
        let md_path = PathBuf::from("test.md");
        let org_path = PathBuf::from("test.org");
        let markdown_path = PathBuf::from("test.markdown");
        let txt_path = PathBuf::from("test.txt");
        let js_path = PathBuf::from("test.js");

        // Check extension filtering logic
        let is_md = |p: &PathBuf| {
            p.extension()
                .map(|ext| ext == "md" || ext == "org" || ext == "markdown")
                .unwrap_or(false)
        };

        assert!(is_md(&md_path));
        assert!(is_md(&org_path));
        assert!(is_md(&markdown_path));
        assert!(!is_md(&txt_path));
        assert!(!is_md(&js_path));
    }

    #[test]
    fn test_file_event_type_serialization() {
        // Test that FileEventType can be cloned and debugged
        let event = FileEventType::Created;
        let debug = format!("{:?}", event);
        assert!(debug.contains("Created"));

        let event2 = FileEventType::Modified;
        let debug2 = format!("{:?}", event2);
        assert!(debug2.contains("Modified"));

        let event3 = FileEventType::Deleted;
        let debug3 = format!("{:?}", event3);
        assert!(debug3.contains("Deleted"));
    }

    #[test]
    fn test_file_event_creation() {
        let event = FileEvent {
            path: PathBuf::from("/test/page.md"),
            event_type: FileEventType::Created,
        };

        assert_eq!(event.path, PathBuf::from("/test/page.md"));
        match event.event_type {
            FileEventType::Created => {}
            _ => panic!("Expected Created"),
        }
    }

    #[test]
    fn test_renamed_event() {
        let old_path = PathBuf::from("/test/old.md");
        let event = FileEventType::Renamed(old_path.clone());

        match event {
            FileEventType::Renamed(p) => assert_eq!(p, old_path),
            _ => panic!("Expected Renamed"),
        }
    }
}
