use std::{env, fs, path::PathBuf};

use serde::Deserialize;
use serde_json::Value;
use thiserror::Error;
use uuid::Uuid;

#[derive(Error, Debug)]
pub enum ConfigError {
    #[error("NotFound")]
    NotFound,
}

#[derive(Deserialize, Clone, PartialEq)]
pub enum Permission {
    Get,
    Update,
    Add,
}

#[derive(Deserialize, Clone, PartialEq)]
pub enum NotionType {
    DataSource,
    Database,
}

#[derive(Deserialize, Clone)]
pub struct NotionConfig {
    pub id: Option<String>,
    pub name: String,
    pub r#type: NotionType,
    pub permission: Vec<Permission>,
}

#[derive(Default, Clone)]
pub struct Config(Vec<NotionConfig>);

impl Config {
    pub fn new() -> Result<Self, ConfigError> {
        let home_dir = dirs::home_dir().unwrap_or("./".into());
        let file_path = PathBuf::from(
            env::var("NOTION_CONFIG")
                .unwrap_or(home_dir.join(".config/notion.json").display().to_string()),
        );
        let config_str = fs::read_to_string(file_path).unwrap_or_default();
        let config: Vec<NotionConfig> = serde_json::from_str(&config_str).unwrap_or_default();

        Ok(Self(config))
    }

    pub fn get_names(&self) -> Vec<String> {
        self.0.iter().map(|c| c.name.clone()).collect()
    }

    pub fn get_id(
        &self,
        name_or_id: &str,
        r#type: NotionType,
        permission: &Permission,
    ) -> Result<String, ConfigError> {
        let id = self
            .0
            .iter()
            .find_map(|c| {
                if c.name == name_or_id && c.r#type == r#type && c.permission.contains(permission) {
                    c.id.clone()
                } else {
                    None
                }
            })
            .unwrap_or(name_or_id.to_string());

        if Uuid::parse_str(&id).is_ok() {
            Ok(id)
        } else {
            Err(ConfigError::NotFound)
        }
    }

    pub fn check_parent(&self, value: Value, permission: &Permission) -> Result<(), ConfigError> {
        let parent = value["parent"].as_object().ok_or(ConfigError::NotFound)?;

        match parent["type"].as_str() {
            Some("data_source_id") => self
                .get_id(
                    parent["data_source_id"]
                        .as_str()
                        .ok_or(ConfigError::NotFound)?,
                    NotionType::DataSource,
                    permission,
                )
                .map(|_| ()),
            Some("database_id") => self
                .get_id(
                    parent["database_id"]
                        .as_str()
                        .ok_or(ConfigError::NotFound)?,
                    NotionType::Database,
                    permission,
                )
                .map(|_| ()),
            _ => Err(ConfigError::NotFound),
        }
    }
}
