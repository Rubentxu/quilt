//! AI Provider implementations — Ollama and OpenAI clients
//!
//! Provides concrete `AIClient` implementations for local Ollama servers
//! and the OpenAI API. These are feature-gated behind `ollama` and `openai`
//! features respectively. When no features are enabled, `MockAIClient` is
//! used as the default (via `create_ai_client`).

use crate::ai_client::{AIClient, AIClientError, AIConfig, AIProvider};
use async_trait::async_trait;
use quilt_domain::entities::Block;
use serde::{Deserialize, Serialize};

#[cfg(any(feature = "ollama", feature = "openai"))]
use reqwest::Client as ReqwestClient;

// ── Ollama Client ─────────────────────────────────────────────────────────────

#[cfg(feature = "ollama")]
mod ollama {
    use super::*;

    /// Configuration for Ollama client
    #[derive(Debug, Clone)]
    pub struct OllamaConfig {
        pub base_url: String,
        pub model: String,
    }

    impl Default for OllamaConfig {
        fn default() -> Self {
            Self {
                base_url: "http://localhost:11434".to_string(),
                model: "llama2".to_string(),
            }
        }
    }

    /// Ollama-specific embedding response
    #[derive(Debug, Deserialize)]
    pub struct OllamaEmbeddingResponse {
        pub embedding: Vec<f32>,
    }

    /// Ollama chat/generate response
    #[derive(Debug, Deserialize)]
    pub struct OllamaGenerateResponse {
        pub response: String,
    }

    /// Ollama chat request
    #[derive(Debug, Serialize)]
    pub struct OllamaChatRequest {
        pub model: String,
        pub messages: Vec<OllamaMessage>,
        pub stream: bool,
    }

    #[derive(Debug, Serialize)]
    pub struct OllamaMessage {
        pub role: String,
        pub content: String,
    }

    /// Ollama chat response
    #[derive(Debug, Deserialize)]
    pub struct OllamaChatResponse {
        pub message: OllamaChatMessage,
    }

    #[derive(Debug, Deserialize)]
    pub struct OllamaChatMessage {
        pub content: String,
    }

    /// Timeout configuration for Ollama requests
    #[derive(Debug, Clone)]
    pub struct OllamaTimeout {
        pub connect_secs: u64,
        pub read_secs: u64,
    }

    impl Default for OllamaTimeout {
        fn default() -> Self {
            Self {
                connect_secs: 10,
                read_secs: 60,
            }
        }
    }

    /// AI client backed by a local Ollama server.
    ///
    /// - `embed()` → POST `/api/embeddings` with `{ "model": ..., "prompt": ... }`
    /// - `bridge_concept()` → POST `/api/generate` with a synthetic prompt
    /// - `chat()` → POST `/api/chat` with messages array
    /// - Dimension: controlled by `AIConfig.dimension` or inferred from first response
    #[derive(Debug)]
    pub struct OllamaClient {
        base_url: String,
        model: String,
        dimension: Option<usize>,
        inferred_dimension: std::sync::Mutex<Option<usize>>,
        timeout: OllamaTimeout,
    }

    impl OllamaClient {
        /// Create a new Ollama client from an `OllamaConfig`.
        pub fn new(config: OllamaConfig) -> Self {
            Self {
                base_url: config.base_url,
                model: config.model,
                dimension: None,
                inferred_dimension: std::sync::Mutex::new(None),
                timeout: OllamaTimeout::default(),
            }
        }

        /// Create a new Ollama client with an `AIConfig`.
        pub fn from_ai_config(config: &AIConfig) -> Self {
            let base_url = config
                .base_url
                .clone()
                .unwrap_or_else(|| "http://localhost:11434".to_string());
            Self {
                base_url,
                model: config.model.clone(),
                dimension: config.dimension,
                inferred_dimension: std::sync::Mutex::new(None),
                timeout: OllamaTimeout::default(),
            }
        }

        /// Set custom timeout configuration.
        pub fn with_timeout(mut self, timeout: OllamaTimeout) -> Self {
            self.timeout = timeout;
            self
        }

        fn client_with_timeout(&self) -> ReqwestClient {
            ReqwestClient::builder()
                .connect_timeout(std::time::Duration::from_secs(self.timeout.connect_secs))
                .read_timeout(std::time::Duration::from_secs(self.timeout.read_secs))
                .build()
                .unwrap_or_else(|_| ReqwestClient::new())
        }

        async fn embed_internal(&self, text: &str) -> Result<Vec<f32>, AIClientError> {
            let url = format!("{}/api/embeddings", self.base_url);
            let body = serde_json::json!({
                "model": self.model,
                "prompt": text
            });

            let client = self.client_with_timeout();
            let resp = client
                .post(&url)
                .json(&body)
                .send()
                .await
                .map_err(|e| AIClientError::Unavailable(e.to_string()))?;

            if !resp.status().is_success() {
                let status = resp.status();
                let text = resp.text().await.unwrap_or_default();
                return Err(AIClientError::Backend(format!(
                    "Ollama error {}: {}",
                    status, text
                )));
            }

            let embedding_resp: OllamaEmbeddingResponse = resp.json().await.map_err(|e| {
                AIClientError::Backend(format!("Failed to parse Ollama response: {}", e))
            })?;

            Ok(embedding_resp.embedding)
        }

        fn validate_or_infer_dimension(&self, embedding: &[f32]) -> Result<(), AIClientError> {
            let actual = embedding.len();
            // Check configured dimension first
            if let Some(expected) = self.dimension {
                if actual != expected {
                    return Err(AIClientError::DimensionMismatch { expected, actual });
                }
                return Ok(());
            }
            // Infer from first successful call
            let mut inferred = self.inferred_dimension.lock().unwrap();
            if inferred.is_none() {
                *inferred = Some(actual);
            } else if inferred.unwrap() != actual {
                return Err(AIClientError::DimensionMismatch {
                    expected: inferred.unwrap(),
                    actual,
                });
            }
            Ok(())
        }
    }

    #[async_trait]
    impl AIClient for OllamaClient {
        async fn embed(&self, text: &str) -> Result<Vec<f32>, AIClientError> {
            let embedding = self.embed_internal(text).await?;
            self.validate_or_infer_dimension(&embedding)?;
            Ok(embedding)
        }

        async fn similarity(&self, a: &str, b: &str) -> Result<f32, AIClientError> {
            let embed_a = self.embed(a).await?;
            let embed_b = self.embed(b).await?;
            Ok(cosine_similarity(&embed_a, &embed_b))
        }

        async fn bridge_concept(
            &self,
            a: &Block,
            b: &Block,
        ) -> Result<Option<String>, AIClientError> {
            let url = format!("{}/api/generate", self.base_url);
            let prompt = format!(
                "Given block A: '{}' and block B: '{}', describe a single conceptual bridge \
                 or connection between them in 1-2 sentences. If no meaningful connection exists, \
                 say 'none'.",
                a.content.chars().take(200).collect::<String>(),
                b.content.chars().take(200).collect::<String>()
            );
            let body = serde_json::json!({
                "model": self.model,
                "prompt": prompt,
                "stream": false
            });

            let client = self.client_with_timeout();
            let resp = client
                .post(&url)
                .json(&body)
                .send()
                .await
                .map_err(|e| AIClientError::Unavailable(e.to_string()))?;

            if !resp.status().is_success() {
                let status = resp.status();
                let text = resp.text().await.unwrap_or_default();
                return Err(AIClientError::Backend(format!(
                    "Ollama generate error {}: {}",
                    status, text
                )));
            }

            let gen_resp: OllamaGenerateResponse = resp.json().await.map_err(|e| {
                AIClientError::Backend(format!("Failed to parse generate response: {}", e))
            })?;

            let response = gen_resp.response.trim();
            if response.eq_ignore_ascii_case("none") || response.is_empty() {
                Ok(None)
            } else {
                Ok(Some(response.to_string()))
            }
        }

        async fn chat(
            &self,
            system_prompt: &str,
            user_prompt: &str,
        ) -> Result<String, AIClientError> {
            let url = format!("{}/api/chat", self.base_url);
            let messages = vec![
                OllamaMessage {
                    role: "system".to_string(),
                    content: system_prompt.to_string(),
                },
                OllamaMessage {
                    role: "user".to_string(),
                    content: user_prompt.to_string(),
                },
            ];
            let body = OllamaChatRequest {
                model: self.model.clone(),
                messages,
                stream: false,
            };

            let client = self.client_with_timeout();
            let resp = client
                .post(&url)
                .json(&body)
                .send()
                .await
                .map_err(|e| AIClientError::Unavailable(e.to_string()))?;

            if !resp.status().is_success() {
                let status = resp.status();
                let text = resp.text().await.unwrap_or_default();
                return Err(AIClientError::Backend(format!(
                    "Ollama chat error {}: {}",
                    status, text
                )));
            }

            let chat_resp: OllamaChatResponse = resp.json().await.map_err(|e| {
                AIClientError::Backend(format!("Failed to parse chat response: {}", e))
            })?;

            Ok(chat_resp.message.content)
        }
    }

    fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
        if a.len() != b.len() {
            return 0.0;
        }
        let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
        let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
        let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
        if norm_a == 0.0 || norm_b == 0.0 {
            return 0.0;
        }
        dot / (norm_a * norm_b)
    }
}

#[cfg(not(feature = "ollama"))]
mod ollama {
    /// Stub when `ollama` feature is not enabled.
    pub struct OllamaClient;
    pub struct OllamaConfig;
}

// ── OpenAI Client ─────────────────────────────────────────────────────────────

#[cfg(feature = "openai")]
mod openai {
    use super::*;

    /// Configuration for OpenAI client
    #[derive(Debug, Clone)]
    pub struct OpenAIConfig {
        pub api_key: String,
        pub base_url: String,
        pub model: String,
    }

    impl Default for OpenAIConfig {
        fn default() -> Self {
            Self {
                api_key: std::env::var("OPENAI_API_KEY").unwrap_or_default(),
                base_url: "https://api.openai.com/v1".to_string(),
                model: "text-embedding-3-small".to_string(),
            }
        }
    }

    /// OpenAI embeddings request
    #[derive(Debug, Serialize)]
    struct OpenAIEmbedRequest {
        model: String,
        input: String,
    }

    /// OpenAI embeddings response
    #[derive(Debug, Deserialize)]
    #[allow(dead_code)]
    struct OpenAIEmbedResponse {
        data: Vec<OpenAIEmbedData>,
        model: String,
    }

    #[derive(Debug, Deserialize)]
    struct OpenAIEmbedData {
        embedding: Vec<f32>,
    }

    /// OpenAI chat completions request
    #[derive(Debug, Serialize)]
    struct OpenAIChatRequest {
        model: String,
        messages: Vec<OpenAIMessage>,
    }

    #[derive(Debug, Serialize)]
    struct OpenAIMessage {
        role: String,
        content: String,
    }

    /// OpenAI chat completions response
    #[derive(Debug, Deserialize)]
    struct OpenAIChatResponse {
        choices: Vec<OpenAIChoice>,
    }

    #[derive(Debug, Deserialize)]
    struct OpenAIChoice {
        message: OpenAIMessageResponse,
    }

    #[derive(Debug, Deserialize)]
    struct OpenAIMessageResponse {
        content: String,
    }

    /// AI client backed by the OpenAI API.
    ///
    /// - `embed()` → POST `/v1/embeddings`
    /// - `bridge_concept()` → POST `/v1/chat/completions`
    #[derive(Debug)]
    pub struct OpenAIClient {
        api_key: String,
        base_url: String,
        model: String,
        client: ReqwestClient,
        dimension: Option<usize>,
        inferred_dimension: std::sync::Mutex<Option<usize>>,
    }

    impl OpenAIClient {
        /// Create a new OpenAI client from an `OpenAIConfig`.
        pub fn new(config: OpenAIConfig) -> Self {
            Self {
                api_key: config.api_key,
                base_url: config.base_url,
                model: config.model,
                client: ReqwestClient::new(),
                dimension: None,
                inferred_dimension: std::sync::Mutex::new(None),
            }
        }

        /// Create a new OpenAI client from an `AIConfig`.
        pub fn from_ai_config(config: &AIConfig) -> Self {
            let api_key = config
                .api_key
                .clone()
                .or_else(|| std::env::var("OPENAI_API_KEY").ok())
                .unwrap_or_default();
            Self {
                api_key,
                base_url: config
                    .base_url
                    .clone()
                    .unwrap_or_else(|| "https://api.openai.com/v1".to_string()),
                model: config.model.clone(),
                client: ReqwestClient::new(),
                dimension: config.dimension,
                inferred_dimension: std::sync::Mutex::new(None),
            }
        }

        async fn embed_internal(&self, text: &str) -> Result<Vec<f32>, AIClientError> {
            let url = format!("{}/embeddings", self.base_url);
            let body = OpenAIEmbedRequest {
                model: self.model.clone(),
                input: text.to_string(),
            };

            let resp = self
                .client
                .post(&url)
                .header("Authorization", format!("Bearer {}", self.api_key))
                .json(&body)
                .send()
                .await
                .map_err(|e| AIClientError::Unavailable(e.to_string()))?;

            let status = resp.status();
            if status.as_u16() == 401 {
                return Err(AIClientError::Backend("401 Unauthorized".to_string()));
            }
            if status.as_u16() == 429 {
                let retry_after = resp
                    .headers()
                    .get("retry-after")
                    .and_then(|v| v.to_str().ok())
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| "N".to_string());
                return Err(AIClientError::Unavailable(format!(
                    "Rate limited, retry after {}s",
                    retry_after
                )));
            }
            if !status.is_success() {
                let text = resp.text().await.unwrap_or_default();
                return Err(AIClientError::Backend(format!(
                    "OpenAI error {}: {}",
                    status, text
                )));
            }

            let embed_resp: OpenAIEmbedResponse = resp.json().await.map_err(|e| {
                AIClientError::Backend(format!("Failed to parse OpenAI response: {}", e))
            })?;

            Ok(embed_resp
                .data
                .into_iter()
                .next()
                .map(|d| d.embedding)
                .unwrap_or_default())
        }

        fn validate_or_infer_dimension(&self, embedding: &[f32]) -> Result<(), AIClientError> {
            let actual = embedding.len();
            if let Some(expected) = self.dimension {
                if actual != expected {
                    return Err(AIClientError::DimensionMismatch { expected, actual });
                }
                return Ok(());
            }
            let mut inferred = self.inferred_dimension.lock().unwrap();
            if inferred.is_none() {
                *inferred = Some(actual);
            } else if inferred.unwrap() != actual {
                return Err(AIClientError::DimensionMismatch {
                    expected: inferred.unwrap(),
                    actual,
                });
            }
            Ok(())
        }
    }

    #[async_trait]
    impl AIClient for OpenAIClient {
        async fn embed(&self, text: &str) -> Result<Vec<f32>, AIClientError> {
            let embedding = self.embed_internal(text).await?;
            self.validate_or_infer_dimension(&embedding)?;
            Ok(embedding)
        }

        async fn similarity(&self, a: &str, b: &str) -> Result<f32, AIClientError> {
            let embed_a = self.embed(a).await?;
            let embed_b = self.embed(b).await?;
            Ok(cosine_similarity(&embed_a, &embed_b))
        }

        async fn bridge_concept(
            &self,
            a: &Block,
            b: &Block,
        ) -> Result<Option<String>, AIClientError> {
            let url = format!("{}/chat/completions", self.base_url);
            let prompt = format!(
                "Given block A: '{}' and block B: '{}', describe a single conceptual bridge \
                 or connection between them in 1-2 sentences. If no meaningful connection exists, \
                 say 'none'.",
                a.content.chars().take(200).collect::<String>(),
                b.content.chars().take(200).collect::<String>()
            );
            let body = OpenAIChatRequest {
                model: self.model.clone(),
                messages: vec![OpenAIMessage {
                    role: "user".to_string(),
                    content: prompt,
                }],
            };

            let resp = self
                .client
                .post(&url)
                .header("Authorization", format!("Bearer {}", self.api_key))
                .json(&body)
                .send()
                .await
                .map_err(|e| AIClientError::Unavailable(e.to_string()))?;

            if !resp.status().is_success() {
                let status = resp.status();
                let text = resp.text().await.unwrap_or_default();
                return Err(AIClientError::Backend(format!(
                    "OpenAI chat error {}: {}",
                    status, text
                )));
            }

            let chat_resp: OpenAIChatResponse = resp.json().await.map_err(|e| {
                AIClientError::Backend(format!("Failed to parse chat response: {}", e))
            })?;

            let response = chat_resp
                .choices
                .into_iter()
                .next()
                .map(|c| c.message.content)
                .unwrap_or_default();

            let response = response.trim();
            if response.eq_ignore_ascii_case("none") || response.is_empty() {
                Ok(None)
            } else {
                Ok(Some(response.to_string()))
            }
        }
    }

    fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
        if a.len() != b.len() {
            return 0.0;
        }
        let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
        let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
        let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
        if norm_a == 0.0 || norm_b == 0.0 {
            return 0.0;
        }
        dot / (norm_a * norm_b)
    }
}

#[cfg(not(feature = "openai"))]
mod openai {
    /// Stub when `openai` feature is not enabled.
    pub struct OpenAIClient;
    pub struct OpenAIConfig;
}

// ── Re-exports for feature-gated types ────────────────────────────────────────

#[cfg(feature = "ollama")]
pub use ollama::OllamaClient;
#[cfg(feature = "ollama")]
pub use ollama::OllamaConfig;

#[cfg(feature = "openai")]
pub use openai::OpenAIClient;
#[cfg(feature = "openai")]
pub use openai::OpenAIConfig;

/// Create an AI client based on the given `AIConfig`.
///
/// Returns a `Box<dyn AIClient>`. When `provider` is `Mock` or when no
/// features for the requested provider are enabled, falls back to `MockAIClient`.
pub fn create_ai_client(config: &AIConfig) -> Box<dyn AIClient> {
    match config.provider {
        #[cfg(feature = "ollama")]
        AIProvider::Ollama => Box::new(OllamaClient::from_ai_config(config)),
        #[cfg(feature = "openai")]
        AIProvider::OpenAI => Box::new(OpenAIClient::from_ai_config(config)),
        _ => Box::new(crate::ai_client::MockAIClient::new()),
    }
}

// ── Cosine similarity (used by both when features enabled) ─────────────────────

#[allow(dead_code)]
fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() {
        return 0.0;
    }
    let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm_a == 0.0 || norm_b == 0.0 {
        return 0.0;
    }
    dot / (norm_a * norm_b)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cosine_similarity_identical() {
        let v = vec![1.0, 0.0, 0.0];
        assert!((cosine_similarity(&v, &v) - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_cosine_similarity_orthogonal() {
        let a = vec![1.0, 0.0, 0.0];
        let b = vec![0.0, 1.0, 0.0];
        assert!((cosine_similarity(&a, &b) - 0.0).abs() < 1e-6);
    }

    #[test]
    fn test_cosine_similarity_different_lengths() {
        let a = vec![1.0, 0.0];
        let b = vec![0.0, 1.0, 0.0];
        assert_eq!(cosine_similarity(&a, &b), 0.0);
    }

    #[test]
    fn test_create_ai_client_returns_dyn() {
        let config = AIConfig::default();
        let client = create_ai_client(&config);
        // Should be able to call embed on the trait object
        let rt = tokio::runtime::Runtime::new().unwrap();
        let result = rt.block_on(client.embed("test"));
        assert!(result.is_ok());
    }

    #[test]
    fn test_create_ai_client_mock_by_default() {
        // Default config has AIProvider::Mock
        let config = AIConfig::default();
        assert_eq!(config.provider, AIProvider::Mock);
        let client = create_ai_client(&config);
        let rt = tokio::runtime::Runtime::new().unwrap();
        // Mock always returns 1536-dim vector
        let embedding = rt.block_on(client.embed("hello")).unwrap();
        assert_eq!(embedding.len(), 1536);
    }
}
