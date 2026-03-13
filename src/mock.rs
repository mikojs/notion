use async_trait::async_trait;
use serde_json::Value;
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::{NotionError, NotionInfo, NotionTrait};

type MockFn<T> = Arc<Mutex<Box<dyn Fn() -> T + Send + Sync>>>;
type MockAsyncFn<T> = Arc<Mutex<Box<dyn Fn() -> Result<T, NotionError> + Send + Sync>>>;

pub struct MockNotion {
    get_list_fn: MockFn<Vec<NotionInfo>>,
    get_data_sources_fn: MockAsyncFn<Vec<Value>>,
    get_database_fn: MockAsyncFn<Value>,
    get_page_fn: MockAsyncFn<Value>,
    add_page_fn: MockAsyncFn<()>,
    update_page_fn: MockAsyncFn<()>,
}

impl Default for MockNotion {
    fn default() -> Self {
        Self::new()
    }
}

impl MockNotion {
    pub fn new() -> Self {
        Self {
            get_list_fn: Arc::new(Mutex::new(Box::new(Vec::new))),
            get_data_sources_fn: Arc::new(Mutex::new(Box::new(|| Ok(vec![])))),
            get_database_fn: Arc::new(Mutex::new(Box::new(|| {
                Err(NotionError::GetFail("Not mocked".to_string()))
            }))),
            get_page_fn: Arc::new(Mutex::new(Box::new(|| {
                Err(NotionError::GetFail("Not mocked".to_string()))
            }))),
            add_page_fn: Arc::new(Mutex::new(Box::new(|| Ok(())))),
            update_page_fn: Arc::new(Mutex::new(Box::new(|| Ok(())))),
        }
    }

    pub async fn mock_get_list<F>(&self, f: F)
    where
        F: Fn() -> Vec<NotionInfo> + Send + Sync + 'static,
    {
        *self.get_list_fn.lock().await = Box::new(f);
    }

    pub async fn mock_get_data_sources<F>(&self, f: F)
    where
        F: Fn() -> Result<Vec<Value>, NotionError> + Send + Sync + 'static,
    {
        *self.get_data_sources_fn.lock().await = Box::new(f);
    }

    pub async fn mock_get_database<F>(&self, f: F)
    where
        F: Fn() -> Result<Value, NotionError> + Send + Sync + 'static,
    {
        *self.get_database_fn.lock().await = Box::new(f);
    }

    pub async fn mock_get_page<F>(&self, f: F)
    where
        F: Fn() -> Result<Value, NotionError> + Send + Sync + 'static,
    {
        *self.get_page_fn.lock().await = Box::new(f);
    }

    pub async fn mock_add_page<F>(&self, f: F)
    where
        F: Fn() -> Result<(), NotionError> + Send + Sync + 'static,
    {
        *self.add_page_fn.lock().await = Box::new(f);
    }

    pub async fn mock_update_page<F>(&self, f: F)
    where
        F: Fn() -> Result<(), NotionError> + Send + Sync + 'static,
    {
        *self.update_page_fn.lock().await = Box::new(f);
    }
}

#[async_trait]
impl NotionTrait for MockNotion {
    fn get_list(&self) -> Vec<NotionInfo> {
        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async { (self.get_list_fn.lock().await)() })
        })
    }

    async fn get_data_sources(
        &self,
        _data_source_name_or_id: &str,
        _filter: &Value,
    ) -> Result<Vec<Value>, NotionError> {
        (self.get_data_sources_fn.lock().await)()
    }

    async fn get_database(&self, _database_name_or_id: &str) -> Result<Value, NotionError> {
        (self.get_database_fn.lock().await)()
    }

    async fn get_page(&self, _page_id: &str) -> Result<Value, NotionError> {
        (self.get_page_fn.lock().await)()
    }

    async fn add_page(&self, _value: Value) -> Result<(), NotionError> {
        (self.add_page_fn.lock().await)()
    }

    async fn update_page(&self, _page_id: &str, _value: Value) -> Result<(), NotionError> {
        (self.update_page_fn.lock().await)()
    }
}
