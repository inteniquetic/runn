use http::{HeaderMap, StatusCode};
use serde_json::Value;
use std::path::Path;
use tracing::info;

use crate::pipeline::config::{load_webhook_token_name, PipelineConfigError};
use crate::pipeline::secrets::load_secret_map;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GitLabEvent {
    Push,
    MergeRequest,
}

#[derive(Debug, PartialEq, Eq)]
pub enum GitLabWebhookError {
    MissingEventHeader,
    InvalidEventHeader,
    UnsupportedEvent(String),
    MissingTokenHeader,
    InvalidTokenHeader,
    InvalidToken,
    MissingWebhookTokenName,
    MissingWebhookTokenValue(String),
    PipelineConfig(PipelineConfigError),
    Secrets(crate::pipeline::secrets::SecretLoadError),
}

#[derive(Debug)]
pub struct GitLabWebhookRequest {
    pub pipeline: String,
    pub event: GitLabEvent,
    pub payload: Value,
}

pub fn handle_webhook(
    headers: &HeaderMap,
    pipeline: &str,
    payload: Value,
    pipeline_path: &Path,
    secrets_path: &Path,
) -> Result<GitLabWebhookRequest, GitLabWebhookError> {
    let token_name = load_webhook_token_name(pipeline_path)
        .map_err(GitLabWebhookError::PipelineConfig)?
        .ok_or(GitLabWebhookError::MissingWebhookTokenName)?;
    let secrets = load_secret_map(secrets_path).map_err(GitLabWebhookError::Secrets)?;
    let token = secrets
        .get(&token_name)
        .ok_or_else(|| GitLabWebhookError::MissingWebhookTokenValue(token_name))?;
    validate_token(headers, token)?;
    let event = event_from_headers(headers)?;

    Ok(GitLabWebhookRequest {
        pipeline: pipeline.to_string(),
        event,
        payload,
    })
}

pub fn event_from_headers(headers: &HeaderMap) -> Result<GitLabEvent, GitLabWebhookError> {
    let Some(raw) = headers.get("X-Gitlab-Event") else {
        return Err(GitLabWebhookError::MissingEventHeader);
    };

    let Ok(value) = raw.to_str() else {
        return Err(GitLabWebhookError::InvalidEventHeader);
    };

    match value {
        "Push Hook" => Ok(GitLabEvent::Push),
        "Merge Request Hook" => Ok(GitLabEvent::MergeRequest),
        _ => Err(GitLabWebhookError::UnsupportedEvent(value.to_string())),
    }
}

pub fn validate_token(headers: &HeaderMap, expected_token: &str) -> Result<(), GitLabWebhookError> {
    let Some(raw) = headers.get("X-Gitlab-Token") else {
        return Err(GitLabWebhookError::MissingTokenHeader);
    };

    let Ok(value) = raw.to_str() else {
        return Err(GitLabWebhookError::InvalidTokenHeader);
    };

    if value == expected_token {
        Ok(())
    } else {
        Err(GitLabWebhookError::InvalidToken)
    }
}

pub fn status_from_error(error: &GitLabWebhookError) -> StatusCode {
    match error {
        GitLabWebhookError::MissingEventHeader | GitLabWebhookError::InvalidEventHeader => {
            StatusCode::BAD_REQUEST
        }
        GitLabWebhookError::UnsupportedEvent(_) => StatusCode::UNPROCESSABLE_ENTITY,
        GitLabWebhookError::MissingTokenHeader
        | GitLabWebhookError::InvalidTokenHeader
        | GitLabWebhookError::InvalidToken
        | GitLabWebhookError::MissingWebhookTokenName
        | GitLabWebhookError::MissingWebhookTokenValue(_) => StatusCode::UNAUTHORIZED,
        GitLabWebhookError::PipelineConfig(_) | GitLabWebhookError::Secrets(_) => {
            StatusCode::INTERNAL_SERVER_ERROR
        }
    }
}

pub fn trigger_pipeline(request: &GitLabWebhookRequest) {
    match request.event {
        GitLabEvent::Push => {
            info!("trigger pipeline {} from push event", request.pipeline);
        }
        GitLabEvent::MergeRequest => {
            info!(
                "trigger pipeline {} from merge_request event",
                request.pipeline
            );
        }
    }

    info!("payload: {}", request.payload);
}
