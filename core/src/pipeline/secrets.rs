use serde::Deserialize;
use std::{collections::HashMap, fs, path::Path};

#[derive(Debug)]
pub enum SecretLoadError {
    Io(std::io::Error),
    Yaml(serde_yaml::Error),
    InvalidKind(String),
    DuplicateName(String),
}

#[derive(Debug, Deserialize)]
struct SecretFile {
    kind: String,
    name: String,
    secrets: Vec<SecretEntry>,
}

#[derive(Debug, Deserialize)]
struct SecretEntry {
    name: String,
    value: String,
}

pub fn load_secret_map(path: &Path) -> Result<HashMap<String, String>, SecretLoadError> {
    let contents = fs::read_to_string(path).map_err(SecretLoadError::Io)?;
    let secrets = parse_secret_file(&contents)?;
    let mut map = HashMap::new();

    for entry in secrets.secrets {
        if map.contains_key(&entry.name) {
            return Err(SecretLoadError::DuplicateName(entry.name));
        }
        map.insert(entry.name, entry.value);
    }

    Ok(map)
}

fn parse_secret_file(contents: &str) -> Result<SecretFile, SecretLoadError> {
    let file: SecretFile = serde_yaml::from_str(contents).map_err(SecretLoadError::Yaml)?;
    if file.kind != "secret" {
        return Err(SecretLoadError::InvalidKind(file.kind));
    }
    Ok(file)
}

pub fn secret_file_name(contents: &str) -> Result<String, SecretLoadError> {
    let file = parse_secret_file(contents)?;
    Ok(file.name)
}
