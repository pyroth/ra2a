//! Request context builder for constructing execution contexts.
//!
//! Provides an abstraction for creating execution contexts
//! with proper task and context ID management.

use async_trait::async_trait;
use std::collections::HashMap;

use crate::error::Result;
use crate::types::{Message, MessageSendParams, Task};

/// Trait for building request contexts.
///
/// Implementations can customize how task IDs and context IDs are
/// generated or retrieved.
#[async_trait]
pub trait ExtendedRequestContextBuilder: Send + Sync {
    /// Builds a request context from message parameters.
    ///
    /// # Arguments
    /// * `params` - The message send parameters
    /// * `existing_task` - An existing task if this is a continuation
    async fn build(
        &self,
        params: &MessageSendParams,
        existing_task: Option<&Task>,
    ) -> Result<ExtendedRequestContext>;
}

/// Simple implementation of ExtendedRequestContextBuilder.
///
/// Generates new UUIDs for task and context IDs when not provided.
#[derive(Debug, Clone, Default)]
pub struct SimpleExtendedRequestContextBuilder;

impl SimpleExtendedRequestContextBuilder {
    /// Creates a new simple context builder.
    pub fn new() -> Self {
        Self
    }

    /// Extracts or generates task ID from message and existing task.
    fn get_task_id(message: &Message, existing_task: Option<&Task>) -> String {
        // Priority: message.task_id > existing_task.id > new UUID
        message
            .task_id
            .clone()
            .or_else(|| existing_task.map(|t| t.id.clone()))
            .unwrap_or_else(|| uuid::Uuid::new_v4().to_string())
    }

    /// Extracts or generates context ID from message and existing task.
    fn get_context_id(message: &Message, existing_task: Option<&Task>) -> String {
        // Priority: message.context_id > existing_task.context_id > new UUID
        message
            .context_id
            .clone()
            .or_else(|| existing_task.map(|t| t.context_id.clone()))
            .unwrap_or_else(|| uuid::Uuid::new_v4().to_string())
    }
}

#[async_trait]
impl ExtendedRequestContextBuilder for SimpleExtendedRequestContextBuilder {
    async fn build(
        &self,
        params: &MessageSendParams,
        existing_task: Option<&Task>,
    ) -> Result<ExtendedRequestContext> {
        let message = &params.message;

        let task_id = Self::get_task_id(message, existing_task);
        let context_id = Self::get_context_id(message, existing_task);

        Ok(ExtendedRequestContext {
            task_id,
            context_id,
            message: message.clone(),
            metadata: params.metadata.clone(),
            extensions: message.extensions.clone(),
        })
    }
}

/// Extended request context with additional information.
#[derive(Debug, Clone)]
pub struct ExtendedRequestContext {
    /// The task ID for this request.
    pub task_id: String,
    /// The context ID for this request.
    pub context_id: String,
    /// The message being processed.
    pub message: Message,
    /// Optional metadata from the request.
    pub metadata: Option<HashMap<String, serde_json::Value>>,
    /// Extensions being used.
    pub extensions: Option<Vec<String>>,
}

impl ExtendedRequestContext {
    /// Creates a new request context.
    pub fn new(
        task_id: impl Into<String>,
        context_id: impl Into<String>,
        message: Message,
    ) -> Self {
        Self {
            task_id: task_id.into(),
            context_id: context_id.into(),
            message,
            metadata: None,
            extensions: None,
        }
    }

    /// Creates a context with auto-generated IDs.
    pub fn create(message: Message) -> Self {
        Self::new(
            uuid::Uuid::new_v4().to_string(),
            uuid::Uuid::new_v4().to_string(),
            message,
        )
    }

    /// Sets the metadata.
    pub fn with_metadata(mut self, metadata: HashMap<String, serde_json::Value>) -> Self {
        self.metadata = Some(metadata);
        self
    }

    /// Sets the extensions.
    pub fn with_extensions(mut self, extensions: Vec<String>) -> Self {
        self.extensions = Some(extensions);
        self
    }

    /// Gets a metadata value by key.
    pub fn get_metadata(&self, key: &str) -> Option<&serde_json::Value> {
        self.metadata.as_ref().and_then(|m| m.get(key))
    }

    /// Checks if a specific extension is enabled.
    pub fn has_extension(&self, uri: &str) -> bool {
        self.extensions
            .as_ref()
            .map(|exts| exts.iter().any(|e| e == uri))
            .unwrap_or(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_simple_builder_new_task() {
        let builder = SimpleExtendedRequestContextBuilder::new();
        let message = Message::user_text("Hello");
        let params = MessageSendParams::new(message);

        let ctx = builder.build(&params, None).await.unwrap();

        assert!(!ctx.task_id.is_empty());
        assert!(!ctx.context_id.is_empty());
    }

    #[tokio::test]
    async fn test_simple_builder_existing_task() {
        let builder = SimpleExtendedRequestContextBuilder::new();
        let existing_task = Task::new("task-123", "ctx-456");
        let message = Message::user_text("Continue");
        let params = MessageSendParams::new(message);

        let ctx = builder.build(&params, Some(&existing_task)).await.unwrap();

        assert_eq!(ctx.task_id, "task-123");
        assert_eq!(ctx.context_id, "ctx-456");
    }

    #[tokio::test]
    async fn test_simple_builder_message_ids_override() {
        let builder = SimpleExtendedRequestContextBuilder::new();
        let existing_task = Task::new("task-123", "ctx-456");
        let message = Message::user_text("Override")
            .with_task_id("new-task")
            .with_context_id("new-ctx");
        let params = MessageSendParams::new(message);

        let ctx = builder.build(&params, Some(&existing_task)).await.unwrap();

        assert_eq!(ctx.task_id, "new-task");
        assert_eq!(ctx.context_id, "new-ctx");
    }

    #[test]
    fn test_context_metadata() {
        let message = Message::user_text("Test");
        let mut metadata = HashMap::new();
        metadata.insert("key".to_string(), serde_json::json!("value"));

        let ctx = ExtendedRequestContext::create(message).with_metadata(metadata);

        assert_eq!(ctx.get_metadata("key"), Some(&serde_json::json!("value")));
        assert_eq!(ctx.get_metadata("missing"), None);
    }

    #[test]
    fn test_context_extensions() {
        let message = Message::user_text("Test");
        let ctx = ExtendedRequestContext::create(message)
            .with_extensions(vec!["urn:a2a:ext:test".to_string()]);

        assert!(ctx.has_extension("urn:a2a:ext:test"));
        assert!(!ctx.has_extension("urn:a2a:ext:other"));
    }
}
