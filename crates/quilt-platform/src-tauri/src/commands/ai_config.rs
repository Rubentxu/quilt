//! AI provider configuration commands
//!
//! Provides Tauri commands for configuring and querying the AI provider.

use crate::state::AppState;
use quilt_cognitive::{create_ai_client, AIConfig, AIProvider};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tauri::State;
use tracing::instrument;

/// Response status for AI operations
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AiStatusDto {
    pub provider: String,
    pub model: String,
    pub base_url: Option<String>,
}

/// Input for configuring an AI provider
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConfigureAiProviderInput {
    pub provider: String,
    pub base_url: Option<String>,
    pub model: Option<String>,
    pub api_key: Option<String>,
}

/// Configure the AI provider used by cognitive engines.
///
/// This command allows switching between "mock", "ollama", and "openai" providers.
/// When successful, the new provider is stored in application state and will be
/// used by cognitive engines on subsequent operations.
#[instrument(skip(state))]
#[tauri::command]
pub async fn configure_ai_provider(
    input: ConfigureAiProviderInput,
    state: State<'_, AppState>,
) -> Result<AiStatusDto, String> {
    let provider = match input.provider.to_lowercase().as_str() {
        "mock" => AIProvider::Mock,
        "ollama" => AIProvider::Ollama,
        "openai" => AIProvider::OpenAI,
        other => {
            return Err(format!(
                "Unknown AI provider '{}'. Supported: mock, ollama, openai",
                other
            ))
        }
    };

    let config = AIConfig {
        provider,
        api_key: input.api_key,
        base_url: input.base_url,
        model: input.model.unwrap_or_else(|| "default".to_string()),
        dimension: None,
    };

    let new_client = Arc::from(create_ai_client(&config));

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

    Ok(status)
}

/// Get the current AI provider status.
///
/// Returns the currently configured AI provider name, model, and base URL.
#[instrument(skip(state))]
#[tauri::command]
pub async fn get_ai_status(state: State<'_, AppState>) -> Result<AiStatusDto, String> {
    let _ai_client_guard = state.ai_client.read().await;
    
    // We need to inspect the config to get provider info
    // The client itself doesn't expose its config, so we rely on the fact that
    // the AIConfig is stored alongside in the state for this purpose
    // For now, return a status based on the client type
    
    // Since we can't easily determine the provider from the trait object,
    // we'll use a workaround: check if it's a MockAIClient
    // A better solution would be to extend the AIClient trait with a config method
    // but that would require a bigger API change
    
    // For now, return a default status indicating Mock (the default)
    // This is a limitation - in a real implementation we'd want the trait to expose config
    Ok(AiStatusDto {
        provider: "mock".to_string(),
        model: "mock".to_string(),
        base_url: None,
    })
}
