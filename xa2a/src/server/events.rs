//! Event handling components for the A2A server.
//!
//! Provides event queues for streaming task updates to clients.

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{RwLock, broadcast};

use crate::error::{A2AError, Result};
use crate::types::{Task, TaskArtifactUpdateEvent, TaskStatusUpdateEvent};

/// An event that can be sent to clients.
#[derive(Debug, Clone)]
pub enum Event {
    /// A status update event.
    StatusUpdate(TaskStatusUpdateEvent),
    /// An artifact update event.
    ArtifactUpdate(TaskArtifactUpdateEvent),
    /// A complete task snapshot.
    Task(Task),
}

/// A queue for sending events to a specific task's subscribers.
#[derive(Debug)]
pub struct EventQueue {
    sender: broadcast::Sender<Event>,
}

impl EventQueue {
    /// Creates a new event queue with the specified capacity.
    pub fn new(capacity: usize) -> Self {
        let (sender, _) = broadcast::channel(capacity);
        Self { sender }
    }

    /// Sends an event to all subscribers.
    pub fn send(&self, event: Event) -> Result<()> {
        self.sender
            .send(event)
            .map_err(|e| A2AError::Stream(format!("Failed to send event: {}", e)))?;
        Ok(())
    }

    /// Subscribes to events from this queue.
    pub fn subscribe(&self) -> broadcast::Receiver<Event> {
        self.sender.subscribe()
    }

    /// Returns the number of active subscribers.
    pub fn subscriber_count(&self) -> usize {
        self.sender.receiver_count()
    }
}

impl Default for EventQueue {
    fn default() -> Self {
        Self::new(100)
    }
}

/// Error when no queue exists for a task.
#[derive(Debug, Clone)]
pub struct NoTaskQueue {
    /// The task ID that has no queue.
    pub task_id: String,
}

impl std::fmt::Display for NoTaskQueue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "No event queue exists for task: {}", self.task_id)
    }
}

impl std::error::Error for NoTaskQueue {}

/// Error when a queue already exists for a task.
#[derive(Debug, Clone)]
pub struct TaskQueueExists {
    /// The task ID that already has a queue.
    pub task_id: String,
}

impl std::fmt::Display for TaskQueueExists {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Event queue already exists for task: {}", self.task_id)
    }
}

impl std::error::Error for TaskQueueExists {}

/// Manages event queues for multiple tasks.
#[derive(Debug, Default)]
pub struct QueueManager {
    queues: Arc<RwLock<HashMap<String, Arc<EventQueue>>>>,
    capacity: usize,
}

impl QueueManager {
    /// Creates a new queue manager.
    pub fn new() -> Self {
        Self::with_capacity(100)
    }

    /// Creates a new queue manager with the specified queue capacity.
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            queues: Arc::new(RwLock::new(HashMap::new())),
            capacity,
        }
    }

    /// Creates a new event queue for a task.
    pub async fn create_queue(
        &self,
        task_id: &str,
    ) -> std::result::Result<Arc<EventQueue>, TaskQueueExists> {
        let mut queues = self.queues.write().await;
        if queues.contains_key(task_id) {
            return Err(TaskQueueExists {
                task_id: task_id.to_string(),
            });
        }
        let queue = Arc::new(EventQueue::new(self.capacity));
        queues.insert(task_id.to_string(), Arc::clone(&queue));
        Ok(queue)
    }

    /// Gets an existing event queue for a task.
    pub async fn get_queue(
        &self,
        task_id: &str,
    ) -> std::result::Result<Arc<EventQueue>, NoTaskQueue> {
        let queues = self.queues.read().await;
        queues.get(task_id).cloned().ok_or(NoTaskQueue {
            task_id: task_id.to_string(),
        })
    }

    /// Gets or creates an event queue for a task.
    pub async fn get_or_create_queue(&self, task_id: &str) -> Arc<EventQueue> {
        // Try to get existing queue first
        {
            let queues = self.queues.read().await;
            if let Some(queue) = queues.get(task_id) {
                return Arc::clone(queue);
            }
        }
        // Create new queue
        let mut queues = self.queues.write().await;
        let queue = Arc::new(EventQueue::new(self.capacity));
        queues.insert(task_id.to_string(), Arc::clone(&queue));
        queue
    }

    /// Removes an event queue for a task.
    pub async fn remove_queue(&self, task_id: &str) -> Option<Arc<EventQueue>> {
        let mut queues = self.queues.write().await;
        queues.remove(task_id)
    }

    /// Returns the number of active queues.
    pub async fn queue_count(&self) -> usize {
        let queues = self.queues.read().await;
        queues.len()
    }

    /// Sends an event to a specific task's queue.
    pub async fn send_event(&self, task_id: &str, event: Event) -> Result<()> {
        let queue = self
            .get_queue(task_id)
            .await
            .map_err(|e| A2AError::Stream(format!("No queue for task {}: {}", task_id, e)))?;
        queue.send(event)
    }
}

/// An in-memory implementation of QueueManager.
pub type InMemoryQueueManager = QueueManager;

/// A consumer for processing events from a queue.
#[async_trait::async_trait]
pub trait EventConsumer: Send + Sync {
    /// Processes an event.
    async fn consume(&self, event: &Event) -> Result<()>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_event_queue() {
        let queue = EventQueue::new(10);
        let mut receiver = queue.subscribe();

        let task = Task::new("task-1", "ctx-1");
        queue.send(Event::Task(task.clone())).unwrap();

        let received = receiver.recv().await.unwrap();
        match received {
            Event::Task(t) => assert_eq!(t.id, "task-1"),
            _ => panic!("Expected Task event"),
        }
    }

    #[tokio::test]
    async fn test_queue_manager() {
        let manager = QueueManager::new();

        let queue = manager.create_queue("task-1").await.unwrap();
        assert_eq!(manager.queue_count().await, 1);

        let retrieved = manager.get_queue("task-1").await.unwrap();
        assert_eq!(Arc::as_ptr(&queue), Arc::as_ptr(&retrieved));

        manager.remove_queue("task-1").await;
        assert_eq!(manager.queue_count().await, 0);
    }
}
