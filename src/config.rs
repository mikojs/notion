use std::{env, str::FromStr};

use regex::{Error as RegexError, Regex};
use strum::ParseError as StrumParseError;
use strum_macros::EnumString;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ConfigError {
    #[error("RegexError: {0}")]
    Regex(#[from] RegexError),
    #[error("ParseError: {0}")]
    StrumParse(#[from] StrumParseError),
}

#[derive(EnumString, Clone)]
pub enum Permission {
    Get,
    Update,
    Add,
}

#[derive(Default, Clone)]
pub struct DataSourceConfig {
    pub name: String,
    pub id: Option<String>,
    pub permission: Option<Permission>,
}

#[derive(Default)]
pub struct Config(Vec<DataSourceConfig>);

impl Config {
    pub fn new() -> Result<Self, ConfigError> {
        let mut data_source_configs: Vec<DataSourceConfig> = Vec::new();
        let data_source_parttern =
            Regex::new(r"^NOTION_DATABASE_(?<name>\w+)_(?<type>ID|PERMISSION)$")?;

        for (key, value) in env::vars() {
            if value.is_empty() {
                continue;
            }

            if let Some(caps) = data_source_parttern.captures(&key) {
                let name = caps["name"].replace("_", "-").to_lowercase();
                let r#type = caps["type"].to_string();

                if let Some(index) = data_source_configs.iter().position(|c| c.name == name) {
                    Config::update_db_config_with_type(
                        &mut data_source_configs[index],
                        r#type,
                        value,
                    )?;
                } else {
                    let mut db_config = DataSourceConfig {
                        name,
                        ..Default::default()
                    };

                    Config::update_db_config_with_type(&mut db_config, r#type, value)?;
                    data_source_configs.push(db_config);
                }
            };
        }

        Ok(Self(data_source_configs))
    }

    fn update_db_config_with_type(
        db_config: &mut DataSourceConfig,
        r#type: String,
        value: String,
    ) -> Result<(), ConfigError> {
        match r#type.as_ref() {
            "ID" => db_config.id = Some(value),
            "PERMISSION" => db_config.permission = Some(Permission::from_str(&value)?),
            _ => unreachable!("unknown type {}", r#type),
        }
        Ok(())
    }
}
