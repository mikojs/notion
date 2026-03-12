use std::{env, str::FromStr};

use regex::{Error as RegexError, Regex};
use strum::ParseError as StrumParseError;
use strum_macros::EnumString;
use thiserror::Error;
use uuid::Uuid;

#[derive(Error, Debug)]
pub enum ConfigError {
    #[error("RegexError: {0}")]
    Regex(#[from] RegexError),
    #[error("ParseError: {0}")]
    StrumParse(#[from] StrumParseError),
    #[error("NotFound")]
    NotFound,
}

#[derive(EnumString, Clone, PartialEq)]
pub enum Permission {
    Get,
    Update,
    Add,
}

#[derive(EnumString, Default, Clone, PartialEq)]
#[strum(serialize_all = "snake_case")]
pub enum NotionType {
    #[default]
    DataSource,
    Database,
}

#[derive(Default, Clone)]
pub struct NotionConfig {
    pub name: String,
    pub r#type: NotionType,
    pub id: Option<String>,
    pub permission: Vec<Permission>,
}

#[derive(Default)]
pub struct Config(Vec<NotionConfig>);

impl Config {
    pub fn new() -> Result<Self, ConfigError> {
        let mut notion_configs: Vec<NotionConfig> = Vec::new();
        let notion_parttern = Regex::new(
            r"^NOTION_(?<type>DATABASE|DATA_SOURCE)_(?<name>\w+)_(?<attr>ID|PERMISSION)$",
        )?;

        for (key, value) in env::vars() {
            if value.is_empty() {
                continue;
            }

            if let Some(caps) = notion_parttern.captures(&key) {
                let name = caps["name"].replace("_", "-").to_lowercase();
                let r#type = caps["type"].replace("_", "-").to_lowercase();
                let attr = caps["attr"].to_string();

                if let Some(index) = notion_configs.iter().position(|c| c.name == name) {
                    Config::update_notion_config_with_type(
                        &mut notion_configs[index],
                        attr,
                        value,
                    )?;
                } else {
                    let mut notion_config = NotionConfig {
                        name,
                        r#type: NotionType::from_str(&r#type)?,
                        ..Default::default()
                    };

                    Config::update_notion_config_with_type(&mut notion_config, attr, value)?;
                    notion_configs.push(notion_config);
                }
            };
        }

        Ok(Self(notion_configs))
    }

    fn update_notion_config_with_type(
        notion_config: &mut NotionConfig,
        attr: String,
        value: String,
    ) -> Result<(), ConfigError> {
        match attr.as_ref() {
            "ID" => notion_config.id = Some(value),
            "PERMISSION" => notion_config.permission.push(Permission::from_str(&value)?),
            _ => unreachable!("unknown type {}", attr),
        }
        Ok(())
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
}
