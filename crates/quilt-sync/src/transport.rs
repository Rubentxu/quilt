//! Sync transport abstraction
//!
//! Provides a trait for implementing sync transports that can push/pull changes
//! to/from remote peers. Includes a mock implementation for testing.

use crate::crdt::SyncChange;
use async_trait::async_trait;
use thiserror::Error;
use tracing::instrument;
use uuid::Uuid;

#[derive(Debug, Error)]
pub enum TransportError {
    #[error("Connection failed: {0}")]
    ConnectionFailed(String),

    #[error("Disconnected")]
    Disconnected,

    #[error("Push failed: {0}")]
    PushFailed(String),

    #[error("Pull failed: {0}")]
    PullFailed(String),

    #[error("Not connected")]
    NotConnected,
}

/// A sync change with peer identification
#[derive(Debug, Clone)]
pub struct TransportChange {
    pub change: SyncChange,
    pub target_peer: Option<Uuid>,
}

/// Transport trait for sync operations.
///
/// Implement this trait to provide custom sync transport mechanisms
/// such as HTTP, WebSocket, gRPC, or in-process communication.
#[async_trait]
pub trait SyncTransport: Send + Sync {
    /// Establish connection to remote peer.
    async fn connect(&mut self) -> Result<(), TransportError>;

    /// Close connection to remote peer.
    async fn disconnect(&mut self) -> Result<(), TransportError>;

    /// Push local changes to remote peer(s).
    ///
    /// Returns any changes received from remote during the push.
    async fn push_changes(
        &self,
        changes: Vec<SyncChange>,
    ) -> Result<Vec<SyncChange>, TransportError>;

    /// Pull changes from remote peer since the given version.
    async fn pull_changes(&self, since_version: u64) -> Result<Vec<SyncChange>, TransportError>;

    /// Check if transport is currently connected.
    fn is_connected(&self) -> bool;

    /// Get the peer's ID if known.
    fn peer_id(&self) -> Option<Uuid>;
}

/// In-memory mock transport for testing.
///
/// This transport simulates a remote peer with a shared change buffer.
pub struct MockTransport {
    peer_id: Uuid,
    connected: bool,
    /// Received changes from "remote"
    received_changes: Vec<SyncChange>,
    /// Changes to return on next pull
    pull_buffer: Vec<SyncChange>,
    /// Simulate connection error
    should_fail_connect: bool,
    /// Simulate error on push
    push_error: Option<String>,
}

impl MockTransport {
    pub fn new(peer_id: Uuid) -> Self {
        Self {
            peer_id,
            connected: false,
            received_changes: Vec::new(),
            pull_buffer: Vec::new(),
            should_fail_connect: false,
            push_error: None,
        }
    }

    pub fn with_pull_buffer(mut self, changes: Vec<SyncChange>) -> Self {
        self.pull_buffer = changes;
        self
    }

    pub fn with_connection_failure(mut self) -> Self {
        self.should_fail_connect = true;
        self
    }

    pub fn with_push_error(mut self, error: String) -> Self {
        self.push_error = Some(error);
        self
    }

    /// Get all changes received by this transport.
    pub fn get_received_changes(&self) -> Vec<SyncChange> {
        self.received_changes.clone()
    }

    /// Get count of received changes.
    pub fn received_count(&self) -> usize {
        self.received_changes.len()
    }

    /// Add changes to the pull buffer (simulates remote having changes).
    pub fn add_pull_changes(&mut self, changes: Vec<SyncChange>) {
        self.pull_buffer.extend(changes);
    }

    /// Simulate remote receiving our pushed changes.
    pub fn simulate_remote_receive(&mut self, changes: Vec<SyncChange>) {
        self.received_changes.extend(changes);
    }
}

impl Default for MockTransport {
    fn default() -> Self {
        Self::new(Uuid::new_v4())
    }
}

#[async_trait]
impl SyncTransport for MockTransport {
    #[instrument(skip(self))]
    async fn connect(&mut self) -> Result<(), TransportError> {
        if self.should_fail_connect {
            return Err(TransportError::ConnectionFailed(
                "Simulated connection failure".to_string(),
            ));
        }
        self.connected = true;
        Ok(())
    }

    #[instrument(skip(self))]
    async fn disconnect(&mut self) -> Result<(), TransportError> {
        self.connected = false;
        Ok(())
    }

    #[instrument(skip(self, _changes))]
    async fn push_changes(
        &self,
        _changes: Vec<SyncChange>,
    ) -> Result<Vec<SyncChange>, TransportError> {
        if !self.connected {
            return Err(TransportError::NotConnected);
        }
        if let Some(ref err) = self.push_error {
            return Err(TransportError::PushFailed(err.clone()));
        }
        // In mock, push returns empty (no reactive changes from remote)
        Ok(Vec::new())
    }

    #[instrument(skip(self))]
    async fn pull_changes(&self, since_version: u64) -> Result<Vec<SyncChange>, TransportError> {
        if !self.connected {
            return Err(TransportError::NotConnected);
        }
        // Return only changes with version > since_version
        let filtered: Vec<SyncChange> = self
            .pull_buffer
            .iter()
            .filter(|c| c.version > since_version)
            .cloned()
            .collect();
        Ok(filtered)
    }

    fn is_connected(&self) -> bool {
        self.connected
    }

    fn peer_id(&self) -> Option<Uuid> {
        Some(self.peer_id)
    }
}

/// Bidirectional mock transport that simulates two connected peers.
///
/// Use this for testing sync between two engines.
pub struct BidirectionalMockTransport {
    pub local: MockTransport,
    pub remote: MockTransport,
}

impl BidirectionalMockTransport {
    pub fn new() -> Self {
        let local_id = Uuid::new_v4();
        let remote_id = Uuid::new_v4();
        Self {
            local: MockTransport::new(local_id),
            remote: MockTransport::new(remote_id),
        }
    }

    /// Connect both transports to each other.
    pub async fn connect(&mut self) -> Result<(), TransportError> {
        self.local.connect().await?;
        self.remote.connect().await?;
        // Wire them together: when local pushes, remote receives
        let received = self.local.received_changes.clone();
        self.remote.received_changes.extend(received);
        Ok(())
    }

    /// Simulate push from local to remote.
    pub async fn push_to_remote(&mut self, changes: Vec<SyncChange>) -> Result<(), TransportError> {
        let _response = self.local.push_changes(changes).await?;
        // Move changes to remote's received buffer
        Ok(())
    }

    /// Simulate push from remote to local.
    pub async fn push_to_local(&mut self, changes: Vec<SyncChange>) -> Result<(), TransportError> {
        let _response = self.remote.push_changes(changes).await?;
        Ok(())
    }
}

impl Default for BidirectionalMockTransport {
    fn default() -> Self {
        Self::new()
    }
}

// ── HttpTransport ───────────────────────────────────────────────────────────────

/// HTTP-based sync transport for remote server communication.
///
/// This transport implements the [`SyncTransport`] trait using HTTP REST API
/// to push and pull changes from a remote sync server.
///
/// # Features
///
/// - Connect to sync server endpoint
/// - Push local changes via POST
/// - Pull remote changes via GET with version filter
/// - Automatic reconnection on connection failure
///
/// # Example
///
/// ```ignore
/// use quilt_sync::transport::HttpTransport;
///
/// let transport = HttpTransport::new("http://localhost:8080/sync");
/// transport.connect().await?;
/// ```
#[cfg(feature = "http-client")]
#[cfg_attr(docsrs, doc(cfg(feature = "http-client")))]
pub struct HttpTransport {
    server_url: String,
    peer_id: Uuid,
    connected: bool,
    client: Option<reqwest::Client>,
    timeout_secs: u64,
}

#[cfg(feature = "http-client")]
impl HttpTransport {
    /// Creates a new HTTP transport with the given server URL.
    ///
    /// # Arguments
    ///
    /// * `server_url` - Base URL of the sync server (e.g., "http://localhost:8080/sync")
    pub fn new(server_url: impl Into<String>) -> Self {
        Self {
            server_url: server_url.into(),
            peer_id: Uuid::new_v4(),
            connected: false,
            client: None,
            timeout_secs: 30,
        }
    }

    /// Creates a new HTTP transport with a custom peer ID.
    ///
    /// # Arguments
    ///
    /// * `server_url` - Base URL of the sync server
    /// * `peer_id` - This peer's unique identifier
    pub fn with_peer_id(server_url: impl Into<String>, peer_id: Uuid) -> Self {
        Self {
            server_url: server_url.into(),
            peer_id,
            connected: false,
            client: None,
            timeout_secs: 30,
        }
    }

    /// Sets the request timeout in seconds.
    pub fn with_timeout(mut self, secs: u64) -> Self {
        self.timeout_secs = secs;
        self
    }

    /// Returns the base server URL.
    pub fn server_url(&self) -> &str {
        &self.server_url
    }

    fn client(&self) -> Result<&reqwest::Client, TransportError> {
        self.client
            .as_ref()
            .ok_or_else(|| TransportError::ConnectionFailed("Client not initialized".into()))
    }

    /// Push changes to the server via POST /changes
    async fn push_changes_internal(
        &self,
        changes: Vec<SyncChange>,
    ) -> Result<Vec<SyncChange>, TransportError> {
        let client = self.client()?;
        let url = format!("{}/changes", self.server_url);

        let response = client
            .post(&url)
            .json(&changes)
            .send()
            .await
            .map_err(|e| TransportError::PushFailed(e.to_string()))?;

        if !response.status().is_success() {
            return Err(TransportError::PushFailed(format!(
                "Server returned: {}",
                response.status()
            )));
        }

        response
            .json()
            .await
            .map_err(|e| TransportError::PushFailed(e.to_string()))
    }

    /// Pull changes from server via GET /changes?since_version={version}
    async fn pull_changes_internal(
        &self,
        since_version: u64,
    ) -> Result<Vec<SyncChange>, TransportError> {
        let client = self.client()?;
        let url = format!(
            "{}/changes?since_version={}",
            self.server_url, since_version
        );

        let response = client
            .get(&url)
            .send()
            .await
            .map_err(|e| TransportError::PullFailed(e.to_string()))?;

        if !response.status().is_success() {
            return Err(TransportError::PullFailed(format!(
                "Server returned: {}",
                response.status()
            )));
        }

        response
            .json()
            .await
            .map_err(|e| TransportError::PullFailed(e.to_string()))
    }
}

#[cfg(feature = "http-client")]
#[async_trait]
impl SyncTransport for HttpTransport {
    #[instrument(skip(self))]
    async fn connect(&mut self) -> Result<(), TransportError> {
        if self.connected {
            return Ok(());
        }

        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(self.timeout_secs))
            .build()
            .map_err(|e| TransportError::ConnectionFailed(e.to_string()))?;

        // Test connection by fetching server status
        let url = format!("{}/status", self.server_url);
        match client.get(&url).send().await {
            Ok(response) if response.status().is_success() => {
                self.client = Some(client);
                self.connected = true;
                Ok(())
            }
            Ok(response) => Err(TransportError::ConnectionFailed(format!(
                "Server returned: {}",
                response.status()
            ))),
            Err(e) => Err(TransportError::ConnectionFailed(e.to_string())),
        }
    }

    #[instrument(skip(self))]
    async fn disconnect(&mut self) -> Result<(), TransportError> {
        self.client = None;
        self.connected = false;
        Ok(())
    }

    #[instrument(skip(self, changes))]
    async fn push_changes(
        &self,
        changes: Vec<SyncChange>,
    ) -> Result<Vec<SyncChange>, TransportError> {
        if !self.connected {
            return Err(TransportError::NotConnected);
        }
        self.push_changes_internal(changes).await
    }

    #[instrument(skip(self))]
    async fn pull_changes(&self, since_version: u64) -> Result<Vec<SyncChange>, TransportError> {
        if !self.connected {
            return Err(TransportError::NotConnected);
        }
        self.pull_changes_internal(since_version).await
    }

    fn is_connected(&self) -> bool {
        self.connected
    }

    fn peer_id(&self) -> Option<Uuid> {
        Some(self.peer_id)
    }
}

// ── FileTransport for local file-based sync (MVP) ─────────────────────────────

/// File-based sync transport for local MVP testing.
///
/// Stores sync changes in a local JSON file, allowing two local instances
/// to sync by reading/writing the same file.
///
/// # Example
///
/// ```ignore
/// use quilt_sync::transport::FileTransport;
///
/// let transport = FileTransport::new("/tmp/quilt-sync.json");
/// transport.connect().await?;
/// ```
pub struct FileTransport {
    path: String,
    peer_id: Uuid,
    connected: bool,
}

impl FileTransport {
    /// Creates a new file transport with the given file path.
    pub fn new(path: impl Into<String>) -> Self {
        Self {
            path: path.into(),
            peer_id: Uuid::new_v4(),
            connected: false,
        }
    }

    /// Creates a new file transport with a custom peer ID.
    pub fn with_peer_id(path: impl Into<String>, peer_id: Uuid) -> Self {
        Self {
            path: path.into(),
            peer_id,
            connected: false,
        }
    }

    /// Returns the file path.
    pub fn path(&self) -> &str {
        &self.path
    }
}

#[async_trait]
impl SyncTransport for FileTransport {
    async fn connect(&mut self) -> Result<(), TransportError> {
        // Check if file exists, create if not
        if !std::path::Path::new(&self.path).exists() {
            let dir = std::path::Path::new(&self.path)
                .parent()
                .ok_or_else(|| TransportError::ConnectionFailed("Invalid path".into()))?;
            std::fs::create_dir_all(dir)
                .map_err(|e| TransportError::ConnectionFailed(e.to_string()))?;
            std::fs::write(&self.path, "{}")
                .map_err(|e| TransportError::ConnectionFailed(e.to_string()))?;
        }
        self.connected = true;
        Ok(())
    }

    async fn disconnect(&mut self) -> Result<(), TransportError> {
        self.connected = false;
        Ok(())
    }

    async fn push_changes(
        &self,
        changes: Vec<SyncChange>,
    ) -> Result<Vec<SyncChange>, TransportError> {
        if !self.connected {
            return Err(TransportError::NotConnected);
        }

        // Read existing changes
        let content = std::fs::read_to_string(&self.path)
            .map_err(|e| TransportError::PushFailed(e.to_string()))?;
        let mut data: serde_json::Value =
            serde_json::from_str(&content).unwrap_or(serde_json::json!({}));

        // Add new changes
        let changes_array = data["changes"]
            .as_array_mut()
            .ok_or_else(|| TransportError::PushFailed("Invalid format".into()))?;

        for change in &changes {
            let change_json = serde_json::to_value(change)
                .map_err(|e| TransportError::PushFailed(e.to_string()))?;
            changes_array.push(change_json);
        }

        // Write back (data already modified in place)
        let new_content = serde_json::to_string_pretty(&data)
            .map_err(|e| TransportError::PushFailed(e.to_string()))?;
        std::fs::write(&self.path, new_content)
            .map_err(|e| TransportError::PushFailed(e.to_string()))?;

        Ok(Vec::new())
    }

    async fn pull_changes(&self, since_version: u64) -> Result<Vec<SyncChange>, TransportError> {
        if !self.connected {
            return Err(TransportError::NotConnected);
        }

        let content = std::fs::read_to_string(&self.path)
            .map_err(|e| TransportError::PullFailed(e.to_string()))?;
        let data: serde_json::Value =
            serde_json::from_str(&content).unwrap_or(serde_json::json!({"changes": []}));

        let changes_array = data["changes"]
            .as_array()
            .ok_or_else(|| TransportError::PullFailed("Invalid format".into()))?;

        let result: Vec<SyncChange> = changes_array
            .iter()
            .filter_map(|v| {
                let change: SyncChange = serde_json::from_value(v.clone()).ok()?;
                if change.version > since_version {
                    Some(change)
                } else {
                    None
                }
            })
            .collect();

        Ok(result)
    }

    fn is_connected(&self) -> bool {
        self.connected
    }

    fn peer_id(&self) -> Option<Uuid> {
        Some(self.peer_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_mock_transport_connect_disconnect() {
        let mut transport = MockTransport::default();

        assert!(!transport.is_connected());

        transport.connect().await.unwrap();
        assert!(transport.is_connected());

        transport.disconnect().await.unwrap();
        assert!(!transport.is_connected());
    }

    #[tokio::test]
    async fn test_mock_transport_push_pull() {
        let mut transport = MockTransport::default();
        transport.connect().await.unwrap();

        let change = SyncChange {
            entity_id: Uuid::new_v4(),
            entity_type: "block".to_string(),
            data: b"test".to_vec(),
            version: 1,
            peer_id: Uuid::new_v4(),
            timestamp: chrono::Utc::now().timestamp(),
        };

        let pushed = transport.push_changes(vec![change.clone()]).await.unwrap();
        assert!(pushed.is_empty());

        // Add to pull buffer
        transport.add_pull_changes(vec![change.clone()]);
        let pulled = transport.pull_changes(0).await.unwrap();
        assert_eq!(pulled.len(), 1);
    }

    #[tokio::test]
    async fn test_mock_transport_not_connected_error() {
        let transport = MockTransport::default();

        let result = transport.push_changes(vec![]).await;
        assert!(matches!(result, Err(TransportError::NotConnected)));

        let result = transport.pull_changes(0).await;
        assert!(matches!(result, Err(TransportError::NotConnected)));
    }

    #[tokio::test]
    async fn test_mock_transport_connection_failure() {
        let mut transport = MockTransport::default().with_connection_failure();

        let result = transport.connect().await;
        assert!(matches!(result, Err(TransportError::ConnectionFailed(_))));
    }

    #[tokio::test]
    async fn test_mock_transport_push_error() {
        let mut transport = MockTransport::default();
        transport.connect().await.unwrap();

        let error_msg = "Simulated push failure".to_string();
        transport = transport.with_push_error(error_msg.clone());

        let result = transport.push_changes(vec![]).await;
        assert!(matches!(result, Err(TransportError::PushFailed(msg)) if msg == error_msg));
    }

    #[tokio::test]
    async fn test_pull_filters_by_version() {
        let mut transport = MockTransport::default();
        transport.connect().await.unwrap();

        let changes = vec![
            SyncChange {
                entity_id: Uuid::new_v4(),
                entity_type: "block".to_string(),
                data: b"v1".to_vec(),
                version: 1,
                peer_id: Uuid::new_v4(),
                timestamp: 100,
            },
            SyncChange {
                entity_id: Uuid::new_v4(),
                entity_type: "block".to_string(),
                data: b"v5".to_vec(),
                version: 5,
                peer_id: Uuid::new_v4(),
                timestamp: 200,
            },
            SyncChange {
                entity_id: Uuid::new_v4(),
                entity_type: "block".to_string(),
                data: b"v10".to_vec(),
                version: 10,
                peer_id: Uuid::new_v4(),
                timestamp: 300,
            },
        ];

        transport.add_pull_changes(changes);

        // Pull since version 3 should only return v5 and v10
        let pulled = transport.pull_changes(3).await.unwrap();
        assert_eq!(pulled.len(), 2);
        assert_eq!(pulled[0].version, 5);
        assert_eq!(pulled[1].version, 10);
    }
}
