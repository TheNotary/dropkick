use serde::Deserialize;
use std::fs;

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

pub fn get_repo_config() -> Result<Config, Box<dyn std::error::Error>> {
    let raw = fs::read_to_string("./.dropkickrc")?;
    let config: Config = serde_yaml::from_str(&raw)?;
    Ok(config)
}
