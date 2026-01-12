use serde::Deserialize;
use std::{fs, path::Path};

#[derive(Debug)]
pub enum PipelineConfigError {
    Io(std::io::Error),
    Yaml(serde_yaml::Error),
    InvalidKind(String),
}

#[derive(Debug, Deserialize)]
struct PipelineConfigFile {
    kind: String,
    name: String,
    #[serde(default)]
    webhook_token: Option<String>,
}

pub fn load_webhook_token_name(path: &Path) -> Result<Option<String>, PipelineConfigError> {
    let contents = fs::read_to_string(path).map_err(PipelineConfigError::Io)?;
    let config: PipelineConfigFile =
        serde_yaml::from_str(&contents).map_err(PipelineConfigError::Yaml)?;
    if config.kind != "pipeline" {
        return Err(PipelineConfigError::InvalidKind(config.kind));
    }
    Ok(config.webhook_token)
}
