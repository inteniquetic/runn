use crate::pipeline::secrets::load_secret_map;
use serde::Deserialize;
use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
    process::Command,
};
use tracing::{info, warn};

#[derive(Debug)]
pub enum PipelineError {
    Io(std::io::Error),
    Yaml(serde_yaml::Error),
    InvalidKind(String),
    Secrets(crate::pipeline::secrets::SecretLoadError),
    StepFailed { step: String, status: i32 },
}

#[derive(Debug, Deserialize)]
struct PipelineFile {
    kind: String,
    name: String,
    steps: Vec<PipelineStep>,
    #[serde(default)]
    on_failure: Option<PipelineFailure>,
}

#[derive(Debug, Deserialize)]
struct PipelineStep {
    name: String,
    commands: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct PipelineFailure {
    commands: Vec<String>,
}

pub fn execute_pipeline(
    pipeline_path: &Path,
    secrets_path: Option<&Path>,
    workdir: Option<&Path>,
) -> Result<(), PipelineError> {
    let pipeline = load_pipeline(pipeline_path)?;
    let secrets = match secrets_path {
        Some(path) => load_secret_map(path).map_err(PipelineError::Secrets)?,
        None => HashMap::new(),
    };

    let base_dir = workdir
        .map(PathBuf::from)
        .or_else(|| pipeline_path.parent().map(PathBuf::from))
        .unwrap_or_else(|| PathBuf::from("."));

    for step in &pipeline.steps {
        info!("running step: {}", step.name);
        if let Err(status) = run_commands(&step.commands, &base_dir, &secrets) {
            warn!("step failed: {} (status {})", step.name, status);
            if let Some(failure) = &pipeline.on_failure {
                let _ = run_commands(&failure.commands, &base_dir, &secrets);
            }
            return Err(PipelineError::StepFailed {
                step: step.name.clone(),
                status,
            });
        }
    }

    Ok(())
}

fn load_pipeline(path: &Path) -> Result<PipelineFile, PipelineError> {
    let contents = fs::read_to_string(path).map_err(PipelineError::Io)?;
    let pipeline: PipelineFile = serde_yaml::from_str(&contents).map_err(PipelineError::Yaml)?;
    if pipeline.kind != "pipeline" {
        return Err(PipelineError::InvalidKind(pipeline.kind));
    }
    Ok(pipeline)
}

fn run_commands(
    commands: &[String],
    base_dir: &Path,
    secrets: &HashMap<String, String>,
) -> Result<(), i32> {
    for command in commands {
        let mut child = Command::new("sh");
        child.arg("-c").arg(command).current_dir(base_dir);
        for (key, value) in secrets {
            child.env(key, value);
        }

        let status = child.status().map_err(|_| 127)?;
        if !status.success() {
            return Err(status.code().unwrap_or(1));
        }
    }
    Ok(())
}
