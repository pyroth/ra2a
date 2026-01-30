//! Task storage traits and implementations.
//!
//! Defines the interface for persisting and retrieving Task objects.

use async_trait::async_trait;

use crate::error::Result;
use crate::types::Task;

use super::RequestContext;

/// Agent Task Store interface.
///
/// Defines the methods for persisting and retrieving `Task` objects.
#[async_trait]
pub trait TaskStore: Send + Sync {
    /// Saves or updates a task in the store.
    async fn save(&self, task: &Task, context: Option<&RequestContext>) -> Result<()>;

    /// Retrieves a task from the store by ID.
    async fn get(&self, task_id: &str, context: Option<&RequestContext>) -> Result<Option<Task>>;

    /// Deletes a task from the store by ID.
    async fn delete(&self, task_id: &str, context: Option<&RequestContext>) -> Result<()>;

    /// Lists all task IDs in the store.
    async fn list_ids(&self, context: Option<&RequestContext>) -> Result<Vec<String>>;
}

/// In-memory implementation of TaskStore.
#[derive(Debug, Default)]
pub struct InMemoryTaskStore {
    tasks: std::sync::Arc<tokio::sync::RwLock<std::collections::HashMap<String, Task>>>,
}

impl InMemoryTaskStore {
    /// Creates a new in-memory task store.
    pub fn new() -> Self {
        Self::default()
    }
}

#[async_trait]
impl TaskStore for InMemoryTaskStore {
    async fn save(&self, task: &Task, _context: Option<&RequestContext>) -> Result<()> {
        let mut tasks = self.tasks.write().await;
        tasks.insert(task.id.clone(), task.clone());
        Ok(())
    }

    async fn get(&self, task_id: &str, _context: Option<&RequestContext>) -> Result<Option<Task>> {
        let tasks = self.tasks.read().await;
        Ok(tasks.get(task_id).cloned())
    }

    async fn delete(&self, task_id: &str, _context: Option<&RequestContext>) -> Result<()> {
        let mut tasks = self.tasks.write().await;
        tasks.remove(task_id);
        Ok(())
    }

    async fn list_ids(&self, _context: Option<&RequestContext>) -> Result<Vec<String>> {
        let tasks = self.tasks.read().await;
        Ok(tasks.keys().cloned().collect())
    }
}

/// Push notification configuration store interface.
#[async_trait]
pub trait PushNotificationConfigStore: Send + Sync {
    /// Saves a push notification configuration.
    async fn save(
        &self,
        task_id: &str,
        config: &crate::types::PushNotificationConfig,
    ) -> Result<()>;

    /// Gets a push notification configuration by task ID and config ID.
    async fn get(
        &self,
        task_id: &str,
        config_id: Option<&str>,
    ) -> Result<Option<crate::types::PushNotificationConfig>>;

    /// Lists all push notification configurations for a task.
    async fn list(&self, task_id: &str) -> Result<Vec<crate::types::PushNotificationConfig>>;

    /// Deletes a push notification configuration.
    async fn delete(&self, task_id: &str, config_id: &str) -> Result<()>;
}

/// In-memory implementation of PushNotificationConfigStore.
#[derive(Debug, Default)]
pub struct InMemoryPushNotificationConfigStore {
    configs: std::sync::Arc<
        tokio::sync::RwLock<
            std::collections::HashMap<String, Vec<crate::types::PushNotificationConfig>>,
        >,
    >,
}

impl InMemoryPushNotificationConfigStore {
    /// Creates a new in-memory push notification config store.
    pub fn new() -> Self {
        Self::default()
    }
}

#[async_trait]
impl PushNotificationConfigStore for InMemoryPushNotificationConfigStore {
    async fn save(
        &self,
        task_id: &str,
        config: &crate::types::PushNotificationConfig,
    ) -> Result<()> {
        let mut configs = self.configs.write().await;
        let task_configs = configs.entry(task_id.to_string()).or_default();

        // Update if exists, otherwise add
        if let Some(existing) = task_configs.iter_mut().find(|c| c.id == config.id) {
            *existing = config.clone();
        } else {
            task_configs.push(config.clone());
        }
        Ok(())
    }

    async fn get(
        &self,
        task_id: &str,
        config_id: Option<&str>,
    ) -> Result<Option<crate::types::PushNotificationConfig>> {
        let configs = self.configs.read().await;
        if let Some(task_configs) = configs.get(task_id) {
            if let Some(cid) = config_id {
                return Ok(task_configs
                    .iter()
                    .find(|c| c.id.as_deref() == Some(cid))
                    .cloned());
            } else {
                return Ok(task_configs.first().cloned());
            }
        }
        Ok(None)
    }

    async fn list(&self, task_id: &str) -> Result<Vec<crate::types::PushNotificationConfig>> {
        let configs = self.configs.read().await;
        Ok(configs.get(task_id).cloned().unwrap_or_default())
    }

    async fn delete(&self, task_id: &str, config_id: &str) -> Result<()> {
        let mut configs = self.configs.write().await;
        if let Some(task_configs) = configs.get_mut(task_id) {
            task_configs.retain(|c| c.id.as_deref() != Some(config_id));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_inmemory_task_store() {
        let store = InMemoryTaskStore::new();
        let task = Task::new("task-1", "ctx-1");

        store.save(&task, None).await.unwrap();

        let retrieved = store.get("task-1", None).await.unwrap();
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().id, "task-1");

        store.delete("task-1", None).await.unwrap();
        let deleted = store.get("task-1", None).await.unwrap();
        assert!(deleted.is_none());
    }
}
