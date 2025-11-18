use serde::Deserialize;
use std::{fs, io};

#[derive(Debug, Deserialize)]
pub struct Project {
    pub name: String,
    // pub version: String,
    #[serde(default)]
    pub template: String,
}

#[derive(Debug, Deserialize)]
pub struct Config {
    pub project: Project,
}

impl Config {
    pub fn new(name: &str) -> Self {
        Config {
            project: Project {
                name: name.to_string(),
                template: "".to_string(),
            },
        }
    }
}

pub fn get_repo_config() -> Config {
    fs::read_to_string("./.dropkickrc")
        .and_then(|raw| {
            serde_yaml::from_str(&raw).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
        })
        .unwrap_or_else(|_| Config::new("Repo Name"))
}
