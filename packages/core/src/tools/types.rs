use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDescriptor {
    pub name: String,
    pub description: String,
    pub input_schema: HashMap<String, serde_json::Value>,
    pub output_schema: Option<HashMap<String, serde_json::Value>>,
    pub required_permissions: Vec<String>,
    pub handler: ToolHandler,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ToolHandler {
    FileRead,
    FileWrite,
    Shell,
    Grep,
    Glob,
    Git,
    WebFetch,
    WebSearch,
    Remember,
    MemoryReplace,
    MemoryRemove,
    RecallArchive,
    AgentSpawn,
    AgentWait,
    AgentResult,
    AgentCancel,
    AgentList,
}

#[derive(Debug, Clone)]
pub struct ToolCall {
    pub id: String,
    pub name: String,
    pub arguments: serde_json::Value,
}

#[derive(Debug, Clone)]
pub enum ToolResult {
    Success(String),
    Error(ToolError),
}

#[derive(Debug, Clone)]
pub struct ToolError {
    pub code: String,
    pub message: String,
    pub suggestion: Option<String>,
}

impl ToolError {
    pub fn new(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
            suggestion: None,
        }
    }

    pub fn with_suggestion(mut self, suggestion: impl Into<String>) -> Self {
        self.suggestion = Some(suggestion.into());
        self
    }
}
