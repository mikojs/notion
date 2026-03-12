use std::env::{self, VarError};

use reqwest::Error as ReqwestError;
use serde_json::Value;
use thiserror::Error;

mod config;

#[derive(Error, Debug)]
pub enum NotionError {
    #[error("ReqwestError: {0}")]
    Reqwest(#[from] ReqwestError),
    #[error("VarError: {0}")]
    Var(#[from] VarError),
    #[error("NoData: {0}")]
    NoData(String),
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
}

impl Notion {
    pub fn new() -> Result<Notion, NotionError> {
        Ok(Self {
            token: env::var("NOTION_TOKEN").map_err(|_| NotionError::NotionTokenNotSet)?,
        })
    }

    async fn format_title(&self, data: &Value) -> Result<Value, NotionError> {
        let title_array = if let Some(title_array) = data["properties"]["Name"]["title"].as_array()
        {
            title_array
        } else if let Some(title_array) = data["title"].as_array() {
            title_array
        } else {
            return Err(NotionError::TitleParseFail(format!(
                "doesn't have title: {}",
                data
            )));
        };

        if title_array.len() == 1 {
            Ok(title_array[0]["plain_text"].clone())
        } else {
            let mut title = "".to_string();

            for title_item in title_array {
                match title_item["type"].as_str() {
                    Some("text") => {
                        title +=
                            title_item["plain_text"]
                                .as_str()
                                .ok_or(NotionError::TitleParseFail(format!(
                                    "parse failed: {}",
                                    title_item
                                )))?
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
                                return Err(NotionError::TitleParseFail(format!(
                                    "parse failed: {}",
                                    title_item
                                )));
                            };

                        if let Ok(page) = page_result {
                            title += Box::pin(self.format_title(&page)).await?.as_str().ok_or(
                                NotionError::TitleParseFail(format!(
                                    "couldn't get the title: {}",
                                    title_item
                                )),
                            )?;
                        } else {
                            title += "[Permission denied page]";
                        }
                        continue;
                    }
                    e => {
                        return Err(NotionError::TitleParseFail(format!(
                            "doesn't support: {:?}",
                            e
                        )));
                    }
                }
            }

            Ok(Value::String(title))
        }
    }

    pub async fn get_data_sources(
        &self,
        data_source_id: &str,
        filter: &Value,
    ) -> Result<Vec<Value>, NotionError> {
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
            None => return Err(NotionError::NoData(data.to_string())),
        };
        let mut output = vec![];

        for mut result in results {
            result["title"] = self.format_title(&result).await?;
            output.push(result);
        }

        Ok(output)
    }

    pub async fn get_database(&self, database_id: &str) -> Result<Value, NotionError> {
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
            return Err(NotionError::GetFail(format!("data: {}", data)));
        }

        Ok(data)
    }

    pub async fn get_page(&self, page_id: &str) -> Result<Value, NotionError> {
        let client = reqwest::Client::new();
        let res = client
            .get(format!("https://api.notion.com/v1/pages/{page_id}",))
            .header("Authorization", format!("Bearer {}", self.token))
            .header("Content-Type", "application/json")
            .header("Notion-Version", "2025-09-03")
            .send()
            .await?;
        let data = res.json::<Value>().await?;

        if !data["id"].is_string() {
            return Err(NotionError::GetFail(format!("data: {}", data)));
        }

        Ok(data)
    }

    pub async fn add_page(&self, value: Value) -> Result<(), NotionError> {
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
            return Err(NotionError::AddFail(format!(
                "data: {}, value: {}",
                data, value
            )));
        }

        Ok(())
    }

    pub async fn update_page(&self, page_id: &str, value: Value) -> Result<(), NotionError> {
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
            return Err(NotionError::UpdateFail(format!(
                "data: {}, value: {}",
                data, value
            )));
        }

        Ok(())
    }
}
