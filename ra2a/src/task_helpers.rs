//! Task helper functions for the A2A SDK.
//!
//! Provides utility functions for creating and manipulating Task objects.
//! This module mirrors Python's `a2a/utils/helpers.py`.
//!
//! # Examples
//!
//! ```rust
//! use ra2a::task_helpers::{create_task_obj, create_user_message, build_text_artifact};
//! use ra2a::types::MessageSendParams;
//!
//! // Create a user message
//! let message = create_user_message("Hello, agent!");
//!
//! // Create a task from message send params
//! let params = MessageSendParams::new(message);
//! let task = create_task_obj(&params);
//!
//! // Create a text artifact
//! let artifact = build_text_artifact("Result content", "artifact-1");
//! ```

use std::collections::HashMap;

use serde_json::Value;
use uuid::Uuid;

use crate::types::{
    AgentCard, Artifact, Message, MessageSendParams, Part, Role, Task, TaskArtifactUpdateEvent,
    TaskState, TaskStatus, TaskStatusUpdateEvent,
};

/// Generates a new UUID v4 string.
pub fn generate_id() -> String {
    Uuid::new_v4().to_string()
}

/// Generates a current timestamp in ISO 8601 format.
pub fn now_iso8601() -> String {
    chrono::Utc::now().to_rfc3339()
}

/// Creates a new task object from message send params.
///
/// Generates UUIDs for task and context IDs if they are not already present.
pub fn create_task_obj(params: &MessageSendParams) -> Task {
    let context_id = params
        .message
        .context_id
        .clone()
        .unwrap_or_else(generate_id);

    Task {
        id: generate_id(),
        context_id,
        status: TaskStatus::new(TaskState::Submitted),
        kind: "task".to_string(),
        history: Some(vec![params.message.clone()]),
        artifacts: None,
        metadata: None,
    }
}

/// Appends artifact data from an event to a task.
///
/// Handles creating the artifacts list if it doesn't exist, adding new artifacts,
/// and appending parts to existing artifacts based on the `append` flag.
pub fn append_artifact_to_task(task: &mut Task, event: &TaskArtifactUpdateEvent) {
    let artifacts = task.artifacts.get_or_insert_with(Vec::new);
    let artifact_id = &event.artifact.artifact_id;
    let append_parts = event.append.unwrap_or(false);

    let existing_idx = artifacts.iter().position(|a| &a.artifact_id == artifact_id);

    if !append_parts {
        // Replace or add new artifact
        if let Some(idx) = existing_idx {
            artifacts[idx] = event.artifact.clone();
        } else {
            artifacts.push(event.artifact.clone());
        }
    } else if let Some(idx) = existing_idx {
        // Append parts to existing artifact
        artifacts[idx].parts.extend(event.artifact.parts.clone());
    }
    // If append=true but artifact doesn't exist, we ignore (matching Python behavior)
}

/// Creates a text artifact with the given content.
pub fn build_text_artifact(text: impl Into<String>, artifact_id: impl Into<String>) -> Artifact {
    Artifact::new(artifact_id, vec![Part::text(text)])
}

/// Checks if server and client output modalities are compatible.
///
/// Returns true if:
/// - Client specifies no preferred output modes
/// - Server specifies no supported output modes
/// - There is at least one common modality
pub fn are_modalities_compatible(
    server_output_modes: Option<&[String]>,
    client_output_modes: Option<&[String]>,
) -> bool {
    match (client_output_modes, server_output_modes) {
        (None, _) | (Some(&[]), _) => true,
        (_, None) | (_, Some(&[])) => true,
        (Some(client), Some(server)) => client.iter().any(|c| server.contains(c)),
    }
}

/// Applies history length limit to a task.
///
/// If `history_length` is specified and the task has history,
/// truncates the history to the specified length from the end.
pub fn apply_history_length(mut task: Task, history_length: Option<i32>) -> Task {
    if let (Some(length), Some(history)) = (history_length, task.history.as_mut()) {
        let length = length.max(0) as usize;
        if history.len() > length {
            let start = history.len() - length;
            *history = history.split_off(start);
        }
    }
    task
}

/// Extracts text content from a message.
///
/// Joins all text parts with the specified delimiter.
pub fn get_message_text(message: &Message, delimiter: &str) -> String {
    message
        .parts
        .iter()
        .filter_map(|p| p.as_text())
        .collect::<Vec<_>>()
        .join(delimiter)
}

/// Creates a data artifact with the given JSON content.
pub fn build_data_artifact(
    data: HashMap<String, Value>,
    artifact_id: impl Into<String>,
) -> Artifact {
    Artifact::new(artifact_id, vec![Part::data(data)])
}

/// Creates an artifact with multiple parts.
pub fn build_multi_part_artifact(parts: Vec<Part>, artifact_id: impl Into<String>) -> Artifact {
    Artifact::new(artifact_id, parts)
}

/// Updates a task's status with a new state.
pub fn update_task_status(task: &mut Task, state: TaskState) {
    task.status = TaskStatus::new(state);
}

/// Updates a task's status with a state and message.
pub fn update_task_status_with_message(task: &mut Task, state: TaskState, message: Message) {
    task.status = TaskStatus::with_message(state, message);
}

/// Appends a message to a task's history.
pub fn append_message_to_task(task: &mut Task, message: Message) {
    task.add_message(message);
}

/// Creates a status update event from a task.
pub fn create_status_update_event(task: &Task, is_final: bool) -> TaskStatusUpdateEvent {
    TaskStatusUpdateEvent::new(&task.id, &task.context_id, task.status.clone(), is_final)
}

/// Creates a simple user message with text content.
pub fn create_user_message(text: impl Into<String>) -> Message {
    Message::new(generate_id(), Role::User, vec![Part::text(text)])
}

/// Creates a simple agent message with text content.
pub fn create_agent_message(text: impl Into<String>) -> Message {
    Message::new(generate_id(), Role::Agent, vec![Part::text(text)])
}

/// Creates a message with a task and context ID.
pub fn create_message_with_context(
    text: impl Into<String>,
    role: Role,
    task_id: impl Into<String>,
    context_id: impl Into<String>,
) -> Message {
    let mut msg = Message::new(generate_id(), role, vec![Part::text(text)]);
    msg.task_id = Some(task_id.into());
    msg.context_id = Some(context_id.into());
    msg
}

/// Recursively removes empty strings, lists and objects from a JSON value.
pub fn clean_empty_values(value: Value) -> Option<Value> {
    match value {
        Value::Object(map) => {
            let cleaned: serde_json::Map<String, Value> = map
                .into_iter()
                .filter_map(|(k, v)| clean_empty_values(v).map(|cleaned| (k, cleaned)))
                .collect();
            if cleaned.is_empty() {
                None
            } else {
                Some(Value::Object(cleaned))
            }
        }
        Value::Array(arr) => {
            let cleaned: Vec<Value> = arr.into_iter().filter_map(clean_empty_values).collect();
            if cleaned.is_empty() {
                None
            } else {
                Some(Value::Array(cleaned))
            }
        }
        Value::String(s) if s.is_empty() => None,
        Value::Null => None,
        other => Some(other),
    }
}

/// Canonicalizes an AgentCard to JSON according to RFC 8785 (JCS).
///
/// Produces a deterministic JSON string with:
/// - Sorted keys
/// - No whitespace
/// - No null values or empty fields
pub fn canonicalize_agent_card(agent_card: &AgentCard) -> Result<String, serde_json::Error> {
    // Serialize to Value first
    let mut value = serde_json::to_value(agent_card)?;

    // Remove signatures field for canonicalization
    if let Value::Object(ref mut map) = value {
        map.remove("signatures");
    }

    // Clean empty values
    let cleaned = clean_empty_values(value).unwrap_or(Value::Object(serde_json::Map::new()));

    // Serialize with sorted keys and no whitespace
    canonicalize_json(&cleaned)
}

/// Produces a canonical JSON string (RFC 8785 JCS).
pub fn canonicalize_json(value: &Value) -> Result<String, serde_json::Error> {
    fn serialize_canonical(value: &Value, output: &mut String) {
        match value {
            Value::Null => output.push_str("null"),
            Value::Bool(b) => output.push_str(if *b { "true" } else { "false" }),
            Value::Number(n) => output.push_str(&n.to_string()),
            Value::String(s) => {
                output.push('"');
                for c in s.chars() {
                    match c {
                        '"' => output.push_str("\\\""),
                        '\\' => output.push_str("\\\\"),
                        '\n' => output.push_str("\\n"),
                        '\r' => output.push_str("\\r"),
                        '\t' => output.push_str("\\t"),
                        c if c.is_control() => {
                            output.push_str(&format!("\\u{:04x}", c as u32));
                        }
                        c => output.push(c),
                    }
                }
                output.push('"');
            }
            Value::Array(arr) => {
                output.push('[');
                for (i, v) in arr.iter().enumerate() {
                    if i > 0 {
                        output.push(',');
                    }
                    serialize_canonical(v, output);
                }
                output.push(']');
            }
            Value::Object(map) => {
                output.push('{');
                // Sort keys for deterministic output
                let mut keys: Vec<_> = map.keys().collect();
                keys.sort();
                for (i, key) in keys.iter().enumerate() {
                    if i > 0 {
                        output.push(',');
                    }
                    output.push('"');
                    output.push_str(key);
                    output.push_str("\":");
                    serialize_canonical(&map[*key], output);
                }
                output.push('}');
            }
        }
    }

    let mut result = String::new();
    serialize_canonical(value, &mut result);
    Ok(result)
}

/// Validates that a task is in a specific state.
pub fn validate_task_state(task: &Task, expected: TaskState) -> bool {
    task.status.state == expected
}

/// Validates that a task is in one of the given states.
pub fn validate_task_states(task: &Task, expected: &[TaskState]) -> bool {
    expected.contains(&task.status.state)
}

/// Checks if a task is complete (in a terminal state).
pub fn is_task_complete(task: &Task) -> bool {
    task.status.state.is_terminal()
}

/// Checks if a task needs user input.
pub fn is_task_waiting_for_input(task: &Task) -> bool {
    matches!(
        task.status.state,
        TaskState::InputRequired | TaskState::AuthRequired
    )
}

/// Extracts the last message from a task's history.
pub fn get_last_message(task: &Task) -> Option<&Message> {
    task.history.as_ref().and_then(|h| h.last())
}

/// Extracts the last user message from a task's history.
pub fn get_last_user_message(task: &Task) -> Option<&Message> {
    task.history
        .as_ref()
        .and_then(|h| h.iter().rev().find(|m| m.role == Role::User))
}

/// Extracts the last agent message from a task's history.
pub fn get_last_agent_message(task: &Task) -> Option<&Message> {
    task.history
        .as_ref()
        .and_then(|h| h.iter().rev().find(|m| m.role == Role::Agent))
}

/// Counts messages in a task's history.
pub fn count_messages(task: &Task) -> usize {
    task.history.as_ref().map(|h| h.len()).unwrap_or(0)
}

/// Counts artifacts in a task.
pub fn count_artifacts(task: &Task) -> usize {
    task.artifacts.as_ref().map(|a| a.len()).unwrap_or(0)
}

/// Gets an artifact by ID from a task.
pub fn get_artifact_by_id<'a>(task: &'a Task, artifact_id: &str) -> Option<&'a Artifact> {
    task.artifacts
        .as_ref()
        .and_then(|arts| arts.iter().find(|a| a.artifact_id == artifact_id))
}

/// Merges metadata from multiple sources.
///
/// Later sources override earlier ones for the same keys.
pub fn merge_metadata(
    sources: Vec<Option<&HashMap<String, Value>>>,
) -> Option<HashMap<String, Value>> {
    let mut result = HashMap::new();
    for source in sources.into_iter().flatten() {
        result.extend(source.clone());
    }
    if result.is_empty() {
        None
    } else {
        Some(result)
    }
}

/// Creates a deep copy of a task with optional modifications.
pub fn clone_task_with_status(task: &Task, new_status: TaskStatus) -> Task {
    let mut cloned = task.clone();
    cloned.status = new_status;
    cloned
}

/// Creates MessageSendParams from a simple text message.
pub fn create_send_params(text: impl Into<String>) -> MessageSendParams {
    MessageSendParams::new(create_user_message(text))
}

/// Creates MessageSendParams with task context.
pub fn create_send_params_with_context(
    text: impl Into<String>,
    task_id: impl Into<String>,
    context_id: impl Into<String>,
) -> MessageSendParams {
    let msg = create_message_with_context(text, Role::User, task_id, context_id);
    MessageSendParams::new(msg)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_id() {
        let id1 = generate_id();
        let id2 = generate_id();
        assert_ne!(id1, id2);
        assert_eq!(id1.len(), 36); // UUID v4 format
    }

    #[test]
    fn test_now_iso8601() {
        let ts = now_iso8601();
        assert!(ts.contains('T'));
        assert!(ts.len() > 20);
    }

    #[test]
    fn test_are_modalities_compatible() {
        // Both None -> compatible
        assert!(are_modalities_compatible(None, None));

        // Client None -> compatible
        assert!(are_modalities_compatible(Some(&["text".to_string()]), None));

        // Common modality -> compatible
        let server = vec!["text".to_string(), "image".to_string()];
        let client = vec!["text".to_string()];
        assert!(are_modalities_compatible(Some(&server), Some(&client)));

        // No common modality -> incompatible
        let server = vec!["image".to_string()];
        let client = vec!["text".to_string()];
        assert!(!are_modalities_compatible(Some(&server), Some(&client)));
    }

    #[test]
    fn test_build_text_artifact() {
        let artifact = build_text_artifact("Hello, world!", "art-1");
        assert_eq!(artifact.artifact_id, "art-1");
        assert_eq!(artifact.parts.len(), 1);
        assert_eq!(artifact.parts[0].as_text(), Some("Hello, world!"));
    }

    #[test]
    fn test_create_user_message() {
        let msg = create_user_message("Hello");
        assert_eq!(msg.role, Role::User);
        assert_eq!(get_message_text(&msg, " "), "Hello");
    }

    #[test]
    fn test_create_agent_message() {
        let msg = create_agent_message("Hi there");
        assert_eq!(msg.role, Role::Agent);
        assert_eq!(get_message_text(&msg, " "), "Hi there");
    }

    #[test]
    fn test_clean_empty_values() {
        let value = serde_json::json!({
            "name": "test",
            "empty": "",
            "null_val": null,
            "nested": {
                "value": "exists",
                "empty_nested": ""
            },
            "empty_array": [],
            "array": ["a", "b"]
        });

        let cleaned = clean_empty_values(value).unwrap();
        let obj = cleaned.as_object().unwrap();

        assert!(obj.contains_key("name"));
        assert!(!obj.contains_key("empty"));
        assert!(!obj.contains_key("null_val"));
        assert!(!obj.contains_key("empty_array"));
        assert!(obj.contains_key("array"));

        let nested = obj.get("nested").unwrap().as_object().unwrap();
        assert!(nested.contains_key("value"));
        assert!(!nested.contains_key("empty_nested"));
    }

    #[test]
    fn test_canonicalize_json() {
        let value = serde_json::json!({
            "z": 1,
            "a": 2,
            "m": {"b": 1, "a": 2}
        });

        let canonical = canonicalize_json(&value).unwrap();
        // Keys should be sorted
        assert!(canonical.starts_with("{\"a\":2"));
        assert!(canonical.contains("\"m\":{\"a\":2,\"b\":1}"));
    }

    #[test]
    fn test_task_state_helpers() {
        let mut task = Task::new("task-1", "ctx-1");

        assert!(validate_task_state(&task, TaskState::Submitted));
        assert!(!is_task_complete(&task));
        assert!(!is_task_waiting_for_input(&task));

        update_task_status(&mut task, TaskState::Completed);
        assert!(is_task_complete(&task));

        update_task_status(&mut task, TaskState::InputRequired);
        assert!(is_task_waiting_for_input(&task));
    }

    #[test]
    fn test_message_helpers() {
        let mut task = Task::new("task-1", "ctx-1");

        let user_msg = create_user_message("Hello");
        append_message_to_task(&mut task, user_msg);

        let agent_msg = create_agent_message("Hi!");
        append_message_to_task(&mut task, agent_msg);

        assert_eq!(count_messages(&task), 2);

        let last_user = get_last_user_message(&task).unwrap();
        assert_eq!(get_message_text(last_user, " "), "Hello");

        let last_agent = get_last_agent_message(&task).unwrap();
        assert_eq!(get_message_text(last_agent, " "), "Hi!");
    }

    #[test]
    fn test_artifact_helpers() {
        let mut task = Task::new("task-1", "ctx-1");

        let artifact = build_text_artifact("Content", "art-1");
        task.add_artifact(artifact);

        assert_eq!(count_artifacts(&task), 1);

        let found = get_artifact_by_id(&task, "art-1");
        assert!(found.is_some());
        assert_eq!(found.unwrap().artifact_id, "art-1");

        let not_found = get_artifact_by_id(&task, "art-999");
        assert!(not_found.is_none());
    }

    #[test]
    fn test_merge_metadata() {
        let mut meta1 = HashMap::new();
        meta1.insert("key1".to_string(), Value::String("value1".to_string()));

        let mut meta2 = HashMap::new();
        meta2.insert("key2".to_string(), Value::String("value2".to_string()));
        meta2.insert("key1".to_string(), Value::String("override".to_string()));

        let merged = merge_metadata(vec![Some(&meta1), Some(&meta2)]).unwrap();

        assert_eq!(merged.get("key1").unwrap(), "override");
        assert_eq!(merged.get("key2").unwrap(), "value2");
    }

    #[test]
    fn test_apply_history_length() {
        let mut task = Task::new("task-1", "ctx-1");

        for i in 0..5 {
            let msg = create_user_message(format!("Message {}", i));
            append_message_to_task(&mut task, msg);
        }

        assert_eq!(count_messages(&task), 5);

        let limited = apply_history_length(task.clone(), Some(3));
        assert_eq!(count_messages(&limited), 3);

        // Should keep the last 3 messages (2, 3, 4)
        let history = limited.history.unwrap();
        assert_eq!(get_message_text(&history[0], " "), "Message 2");
        assert_eq!(get_message_text(&history[2], " "), "Message 4");
    }
}
