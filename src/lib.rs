use async_trait::async_trait;
use reqwest::Error as ReqwestError;
use serde_json::Value;
use std::env::{self, VarError};
use thiserror::Error;

pub use crate::config::NotionInfo;
use crate::config::{Config, ConfigError, NotionType, Permission};

mod config;
#[cfg(feature = "test-utils")]
pub mod mock;

#[async_trait]
pub trait NotionTrait: Send + Sync {
    fn get_list(&self) -> Vec<NotionInfo>;
    async fn get_data_sources(
        &self,
        data_source_name_or_id: &str,
        filter: &Value,
    ) -> Result<Vec<Value>, NotionError>;
    async fn get_database(&self, database_name_or_id: &str) -> Result<Value, NotionError>;
    async fn get_page(&self, page_id: &str) -> Result<Value, NotionError>;
    async fn add_page(&self, value: Value) -> Result<(), NotionError>;
    async fn update_page(&self, page_id: &str, value: Value) -> Result<(), NotionError>;
}

#[derive(Error, Debug)]
pub enum NotionError {
    #[error("ReqwestError: {0}")]
    Reqwest(#[from] ReqwestError),
    #[error("VarError: {0}")]
    Var(#[from] VarError),
    #[error("ConfigError: {0}")]
    Config(#[from] ConfigError),
    #[error("AddFail: {0}")]
    AddFail(String),
    #[error("GetFail: {0}")]
    GetFail(String),
    #[error("UpdateFail: {0}")]
    UpdateFail(String),
    #[error("TitleParseFail: {0}")]
    TitleParseFail(String),
    #[error("NOTION_TOKEN env var is not set")]
    NotionTokenNotSet,
}

#[derive(Clone)]
pub struct Notion {
    token: String,
    config: Config,
}

impl Notion {
    pub fn new() -> Result<Notion, NotionError> {
        Ok(Self {
            token: env::var("NOTION_TOKEN").map_err(|_| NotionError::NotionTokenNotSet)?,
            config: Config::new()?,
        })
    }

    async fn format_title(&self, data: &Value) -> Result<Value, NotionError> {
        let title_array = if let Some(title_array) = data["properties"]["Name"]["title"].as_array()
        {
            title_array
        } else if let Some(title_array) = data["title"].as_array() {
            title_array
        } else {
            return Err(NotionError::TitleParseFail("Title not found".to_string()));
        };

        if title_array.len() == 1 {
            Ok(title_array[0]["plain_text"].clone())
        } else {
            let mut title = "".to_string();

            for title_item in title_array {
                match title_item["type"].as_str() {
                    Some("text") => {
                        title += title_item["plain_text"]
                            .as_str()
                            .ok_or(NotionError::TitleParseFail("Not plain text".to_string()))?
                    }
                    Some("mention") => {
                        let page_result =
                            if let Some(page_id) = title_item["mention"]["page"]["id"].as_str() {
                                self.get_page(page_id).await
                            } else if let Some(database_id) =
                                title_item["mention"]["database"]["id"].as_str()
                            {
                                self.get_database(database_id).await
                            } else {
                                return Err(NotionError::TitleParseFail(
                                    "Not found database".to_string(),
                                ));
                            };

                        if let Ok(page) = page_result {
                            title += Box::pin(self.format_title(&page)).await?.as_str().ok_or(
                                NotionError::TitleParseFail("Not found page's Title".to_string()),
                            )?;
                        } else {
                            title += "[Permission denied page]";
                        }
                        continue;
                    }
                    e => {
                        return Err(NotionError::TitleParseFail(format!(
                            "Doesn't support: {:?}",
                            e
                        )));
                    }
                }
            }

            Ok(Value::String(title))
        }
    }
}

#[async_trait]
impl NotionTrait for Notion {
    fn get_list(&self) -> Vec<NotionInfo> {
        self.config.get_list()
    }

    async fn get_data_sources(
        &self,
        data_source_name_or_id: &str,
        filter: &Value,
    ) -> Result<Vec<Value>, NotionError> {
        let data_source_id = self.config.get_id(
            data_source_name_or_id,
            NotionType::DataSource,
            &Permission::Get,
        )?;
        let client = reqwest::Client::new();
        let res = client
            .post(format!(
                "https://api.notion.com/v1/data_sources/{data_source_id}/query",
            ))
            .header("Authorization", format!("Bearer {}", self.token))
            .header("Content-Type", "application/json")
            .header("Notion-Version", "2025-09-03")
            .json(filter)
            .send()
            .await?;
        let data = res.json::<Value>().await?;
        let results = match data["results"].as_array() {
            Some(results) => results.to_vec(),
            None => return Err(NotionError::GetFail("Data Source".to_string())),
        };
        let mut output = vec![];

        for mut result in results {
            result["title"] = self.format_title(&result).await?;
            output.push(result);
        }

        Ok(output)
    }

    async fn get_database(&self, database_name_or_id: &str) -> Result<Value, NotionError> {
        let database_id =
            self.config
                .get_id(database_name_or_id, NotionType::Database, &Permission::Get)?;
        let client = reqwest::Client::new();
        let res = client
            .get(format!("https://api.notion.com/v1/databases/{database_id}",))
            .header("Authorization", format!("Bearer {}", self.token))
            .header("Content-Type", "application/json")
            .header("Notion-Version", "2025-09-03")
            .send()
            .await?;
        let data = res.json::<Value>().await?;

        if !data["id"].is_string() {
            return Err(NotionError::GetFail("Database".to_string()));
        }

        Ok(data)
    }

    async fn get_page(&self, page_id: &str) -> Result<Value, NotionError> {
        let client = reqwest::Client::new();
        let res = client
            .get(format!("https://api.notion.com/v1/pages/{page_id}",))
            .header("Authorization", format!("Bearer {}", self.token))
            .header("Content-Type", "application/json")
            .header("Notion-Version", "2025-09-03")
            .send()
            .await?;
        let data = res.json::<Value>().await?;

        if !data["id"].is_string()
            || self
                .config
                .check_parent(data.clone(), &Permission::Get)
                .is_err()
        {
            return Err(NotionError::GetFail("Page".to_string()));
        }

        Ok(data)
    }

    async fn add_page(&self, value: Value) -> Result<(), NotionError> {
        if self
            .config
            .check_parent(value.clone(), &Permission::Add)
            .is_err()
        {
            return Err(NotionError::AddFail("Page".to_string()));
        }

        let client = reqwest::Client::new();
        let res = client
            .post("https://api.notion.com/v1/pages")
            .header("Authorization", format!("Bearer {}", self.token))
            .header("Content-Type", "application/json")
            .header("Notion-Version", "2025-09-03")
            .json(&value)
            .send()
            .await?;
        let data = res.json::<Value>().await?;

        if !data["id"].is_string() {
            return Err(NotionError::AddFail("Page".to_string()));
        }

        Ok(())
    }

    async fn update_page(&self, page_id: &str, value: Value) -> Result<(), NotionError> {
        let page = self.get_page(page_id).await?;

        if self
            .config
            .check_parent(page.clone(), &Permission::Update)
            .is_err()
        {
            return Err(NotionError::UpdateFail("Page".to_string()));
        }

        let client = reqwest::Client::new();
        let res = client
            .patch(format!("https://api.notion.com/v1/pages/{page_id}",))
            .header("Authorization", format!("Bearer {}", self.token))
            .header("Content-Type", "application/json")
            .header("Notion-Version", "2025-09-03")
            .json(&value)
            .send()
            .await?;
        let data = res.json::<Value>().await?;

        if !data["id"].is_string() {
            return Err(NotionError::UpdateFail("Page".to_string()));
        }

        Ok(())
    }
}
