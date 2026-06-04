//! AI configuration HTTP handlers

use axum::{Json, extract::Extension};
use axum::{Router, routing::get};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::instrument;

use crate::error::AppError;
use crate::state::AppState;
use quilt_cognitive::{AIConfig, AIProvider, ai_client::MockAIClient};

/// Response status for AI operations
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AiStatusDto {
    pub provider: String,
    pub model: String,
    pub base_url: Option<String>,
}

/// Input for configuring an AI provider
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConfigureAiProviderInput {
    pub provider: String,
    pub base_url: Option<String>,
    pub model: Option<String>,
    pub api_key: Option<String>,
}

/// Create router for /api/v1/ai-config
pub fn routes() -> Router {
    Router::new().route("/status", get(get_ai_status).post(configure_ai_provider))
}

/// GET /api/v1/ai-config/status
#[instrument(skip(state))]
pub async fn get_ai_status(
    Extension(state): Extension<AppState>,
) -> Result<Json<AiStatusDto>, AppError> {
    let _ = state;
    Ok(Json(AiStatusDto {
        provider: "mock".to_string(),
        model: "mock".to_string(),
        base_url: None,
    }))
}

/// POST /api/v1/ai-config/status
#[instrument(skip(state))]
pub async fn configure_ai_provider(
    Extension(state): Extension<AppState>,
    Json(input): Json<ConfigureAiProviderInput>,
) -> Result<Json<AiStatusDto>, AppError> {
    let provider = match input.provider.to_lowercase().as_str() {
        "mock" => AIProvider::Mock,
        "ollama" => AIProvider::Ollama,
        "openai" => AIProvider::OpenAI,
        other => {
            return Err(AppError::BadRequest(format!(
                "Unknown AI provider '{}'. Supported: mock, ollama, openai",
                other
            )));
        }
    };

    let config = AIConfig {
        provider,
        api_key: input.api_key,
        base_url: input.base_url,
        model: input.model.unwrap_or_else(|| "default".to_string()),
        dimension: None,
    };

    // For now, only Mock is fully supported without feature flags
    let new_client: Arc<dyn quilt_cognitive::AIClient> = if config.provider == AIProvider::Mock {
        Arc::new(MockAIClient::new())
    } else {
        // TODO: Support ollama/openai when their features are enabled
        return Err(AppError::BadRequest(
            "Only 'mock' provider is fully supported. Enable 'ollama' or 'openai' features for other providers.".to_string()
        ));
    };

    let mut ai_client_guard = state.ai_client.write().await;
    *ai_client_guard = new_client;

    let status = AiStatusDto {
        provider: input.provider,
        model: config.model,
        base_url: config.base_url,
    };

    tracing::info!(
        provider = %status.provider,
        model = %status.model,
        "AI provider configured"
    );

    Ok(Json(status))
}
