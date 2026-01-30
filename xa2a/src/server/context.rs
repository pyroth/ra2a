//! Server-side context management.

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::types::Task;

/// Thread-safe storage for managing request context.
#[derive(Debug, Default)]
pub struct RequestContext {
    /// Request-scoped data.
    data: HashMap<String, serde_json::Value>,
}

impl RequestContext {
    /// Creates a new request context.
    pub fn new() -> Self {
        Self::default()
    }

    /// Gets a value from the context.
    pub fn get(&self, key: &str) -> Option<&serde_json::Value> {
        self.data.get(key)
    }

    /// Sets a value in the context.
    pub fn set(&mut self, key: impl Into<String>, value: serde_json::Value) {
        self.data.insert(key.into(), value);
    }

    /// Removes a value from the context.
    pub fn remove(&mut self, key: &str) -> Option<serde_json::Value> {
        self.data.remove(key)
    }
}

/// Server state shared across all request handlers.
#[derive(Debug)]
pub struct ServerState<E> {
    /// The agent executor.
    pub executor: Arc<E>,
    /// Task storage.
    pub tasks: Arc<RwLock<HashMap<String, Task>>>,
}

impl<E> ServerState<E> {
    /// Creates a new server state.
    pub fn new(executor: E) -> Self {
        Self {
            executor: Arc::new(executor),
            tasks: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Gets a task by ID.
    pub async fn get_task(&self, task_id: &str) -> Option<Task> {
        let tasks = self.tasks.read().await;
        tasks.get(task_id).cloned()
    }

    /// Stores a task.
    pub async fn store_task(&self, task: Task) {
        let mut tasks = self.tasks.write().await;
        tasks.insert(task.id.clone(), task);
    }

    /// Removes a task.
    pub async fn remove_task(&self, task_id: &str) -> Option<Task> {
        let mut tasks = self.tasks.write().await;
        tasks.remove(task_id)
    }

    /// Lists all task IDs.
    pub async fn list_task_ids(&self) -> Vec<String> {
        let tasks = self.tasks.read().await;
        tasks.keys().cloned().collect()
    }
}

impl<E> Clone for ServerState<E> {
    fn clone(&self) -> Self {
        Self {
            executor: Arc::clone(&self.executor),
            tasks: Arc::clone(&self.tasks),
        }
    }
}
