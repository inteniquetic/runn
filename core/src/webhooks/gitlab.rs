use http::{HeaderMap, StatusCode};
use serde_json::Value;
use tracing::info;

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
    MissingTokenEnv,
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
) -> Result<GitLabWebhookRequest, GitLabWebhookError> {
    let token = expected_token_from_env()?;
    validate_token(headers, &token)?;
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
        | GitLabWebhookError::MissingTokenEnv => StatusCode::UNAUTHORIZED,
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

fn expected_token_from_env() -> Result<String, GitLabWebhookError> {
    match std::env::var("GITLAB_WEBHOOK_TOKEN") {
        Ok(token) if !token.is_empty() => Ok(token),
        _ => Err(GitLabWebhookError::MissingTokenEnv),
    }
}
