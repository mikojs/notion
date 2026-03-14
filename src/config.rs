use std::{env, fs, path::PathBuf};

use serde::{Deserialize, Serialize};
use serde_json::Value;
use strum_macros::Display;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ConfigError {
    #[error("Not found")]
    NotFound,
    #[error("Permssion denied: {0}")]
    PermissionDenied(String),
}

#[derive(Deserialize, Serialize, Display, Clone, PartialEq)]
pub enum Permission {
    Get,
    Update,
    Add,
}

#[derive(Deserialize, Serialize, Clone, PartialEq)]
pub enum NotionType {
    DataSource,
    Database,
}

#[derive(Deserialize, Clone)]
pub struct NotionConfig {
    pub id: String,
    pub name: String,
    pub r#type: NotionType,
    pub permission: Vec<Permission>,
}

#[derive(Serialize)]
pub struct NotionInfo {
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

    pub fn get_list(&self) -> Vec<NotionInfo> {
        self.0
            .iter()
            .map(|c| NotionInfo {
                name: c.name.clone(),
                r#type: c.r#type.clone(),
                permission: c.permission.clone(),
            })
            .collect()
    }

    pub fn get_id(
        &self,
        name_or_id: &str,
        r#type: NotionType,
        permission: &Permission,
    ) -> Result<String, ConfigError> {
        self.0
            .iter()
            .find_map(|c| {
                if (c.id == name_or_id || c.name == name_or_id)
                    && c.r#type == r#type
                    && c.permission.contains(permission)
                {
                    Some(c.id.clone())
                } else {
                    None
                }
            })
            .ok_or(ConfigError::PermissionDenied(permission.to_string()))
    }

    pub fn get_parent_id(
        &self,
        value: Value,
        permission: &Permission,
    ) -> Result<(String, String), ConfigError> {
        let parent = value["parent"]
            .as_object()
            .ok_or(ConfigError::PermissionDenied(
                "parent id not found".to_string(),
            ))?;

        match parent["type"].as_str() {
            Some("data_source_id") => Ok((
                "data_source_id".to_string(),
                self.get_id(
                    parent["data_source_id"]
                        .as_str()
                        .ok_or(ConfigError::NotFound)?,
                    NotionType::DataSource,
                    permission,
                )?,
            )),
            Some("database_id") => Ok((
                "database_id".to_string(),
                self.get_id(
                    parent["database_id"]
                        .as_str()
                        .ok_or(ConfigError::NotFound)?,
                    NotionType::Database,
                    permission,
                )?,
            )),
            _ => Err(ConfigError::PermissionDenied(permission.to_string())),
        }
    }
}
