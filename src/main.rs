use notion::{Notion, NotionError};
use rmcp::{
    ErrorData as McpError, RmcpError, ServerHandler, ServiceExt,
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    model::*,
    schemars::JsonSchema,
    service::ServerInitializeError,
    tool, tool_handler, tool_router,
    transport::io::stdio,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::Arc;
use thiserror::Error;
use tokio::{sync::Mutex, task::JoinError};

#[derive(Error, Debug)]
pub enum ServerError {
    #[error("NotionError: {0}")]
    Notion(#[from] NotionError),
    #[error("RmcpError: {0}")]
    Rmcp(#[from] RmcpError),
    #[error("ServerInitializeError: {0}")]
    ServerInitialize(#[from] ServerInitializeError),
    #[error("JoinError: {0}")]
    Join(#[from] JoinError),
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct GetDataSourcesParams {
    /// The name or ID of the data source
    pub data_source_name_or_id: String,
    /// Optional filter object for the query
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filter: Option<Value>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct GetDatabaseParams {
    /// The name or ID of the database
    pub database_name_or_id: String,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct GetPageParams {
    /// The ID of the page
    pub page_id: String,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct AddPageParams {
    /// The page data to create (must include parent and properties)
    pub value: Value,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct UpdatePageParams {
    /// The ID of the page to update
    pub page_id: String,
    /// The page data to update
    pub value: Value,
}

#[derive(Clone)]
pub struct NotionServer {
    notion: Arc<Mutex<Notion>>,
    tool_router: ToolRouter<NotionServer>,
}

#[tool_router]
impl NotionServer {
    pub fn new(notion: Notion) -> Self {
        Self {
            notion: Arc::new(Mutex::new(notion)),
            tool_router: Self::tool_router(),
        }
    }

    #[tool(description = "List all Notion names")]
    async fn list(&self) -> Result<CallToolResult, McpError> {
        let notion = self.notion.lock().await;

        Ok(CallToolResult::success(vec![Content::json(
            serde_json::json!({ "results": notion.list() }),
        )?]))
    }

    #[tool(description = "Query data sources from Notion with optional filter")]
    async fn get_data_sources(
        &self,
        Parameters(params): Parameters<GetDataSourcesParams>,
    ) -> Result<CallToolResult, McpError> {
        let filter = params.filter.unwrap_or(serde_json::json!({}));
        let notion = self.notion.lock().await;

        match notion
            .get_data_sources(&params.data_source_name_or_id, &filter)
            .await
        {
            Ok(results) => Ok(CallToolResult::success(vec![Content::json(
                serde_json::json!({ "results": results }),
            )?])),
            Err(e) => Err(McpError::internal_error(e.to_string(), None)),
        }
    }

    #[tool(description = "Get a Notion database by name or ID")]
    async fn get_database(
        &self,
        Parameters(params): Parameters<GetDatabaseParams>,
    ) -> Result<CallToolResult, McpError> {
        let notion = self.notion.lock().await;

        match notion.get_database(&params.database_name_or_id).await {
            Ok(data) => Ok(CallToolResult::success(vec![Content::json(data)?])),
            Err(e) => Err(McpError::internal_error(e.to_string(), None)),
        }
    }

    #[tool(description = "Get a Notion page by ID")]
    async fn get_page(
        &self,
        Parameters(params): Parameters<GetPageParams>,
    ) -> Result<CallToolResult, McpError> {
        let notion = self.notion.lock().await;

        match notion.get_page(&params.page_id).await {
            Ok(data) => Ok(CallToolResult::success(vec![Content::json(data)?])),
            Err(e) => Err(McpError::internal_error(e.to_string(), None)),
        }
    }

    #[tool(description = "Create a new page in Notion")]
    async fn add_page(
        &self,
        Parameters(params): Parameters<AddPageParams>,
    ) -> Result<CallToolResult, McpError> {
        let notion = self.notion.lock().await;

        match notion.add_page(params.value).await {
            Ok(()) => Ok(CallToolResult::success(vec![Content::text(
                "Page created successfully",
            )])),
            Err(e) => Err(McpError::internal_error(e.to_string(), None)),
        }
    }

    #[tool(description = "Update an existing Notion page")]
    async fn update_page(
        &self,
        Parameters(params): Parameters<UpdatePageParams>,
    ) -> Result<CallToolResult, McpError> {
        let notion = self.notion.lock().await;

        match notion.update_page(&params.page_id, params.value).await {
            Ok(()) => Ok(CallToolResult::success(vec![Content::text(
                "Page updated successfully",
            )])),
            Err(e) => Err(McpError::internal_error(e.to_string(), None)),
        }
    }
}

#[tool_handler]
impl ServerHandler for NotionServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo::new(ServerCapabilities::builder().enable_tools().build())
            .with_server_info(Implementation::from_build_env())
            .with_instructions(
                "Notion MCP Server - Access Notion databases, pages, and data sources. \
                Tools: list, get_data_sources, get_database, get_page, add_page, update_page."
                    .to_string(),
            )
    }
}

#[allow(clippy::result_large_err)]
#[tokio::main]
async fn main() -> Result<(), ServerError> {
    let notion = Notion::new()?;
    let server = NotionServer::new(notion);
    let transport = stdio();
    let service = server.serve(transport).await?;

    service.waiting().await?;

    Ok(())
}
