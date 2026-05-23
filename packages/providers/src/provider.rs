use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tokio::sync::mpsc;

#[async_trait]
pub trait LLMProvider: Send + Sync {
    async fn chat_completions(
        &self,
        messages: Vec<Message>,
        model: &str,
        options: ChatOptions,
    ) -> anyhow::Result<ChatResponse>;

    async fn chat_completions_stream(
        &self,
        messages: Vec<Message>,
        model: &str,
        options: ChatOptions,
    ) -> anyhow::Result<mpsc::UnboundedReceiver<anyhow::Result<StreamChunk>>> {
        let response = self.chat_completions(messages, model, options).await?;
        let (tx, rx) = mpsc::unbounded_channel();
        if let Some(choice) = response.choices.into_iter().next() {
            let _ = tx.send(Ok(StreamChunk {
                delta: StreamDelta::Content(choice.message.content),
            }));
        }
        let _ = tx.send(Ok(StreamChunk {
            delta: StreamDelta::Done,
        }));
        Ok(rx)
    }

    async fn list_models(&self) -> anyhow::Result<Vec<ModelInfo>>;

    async fn health_check(&self) -> bool;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: MessageRole,
    pub content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ToolCall>>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum MessageRole {
    System,
    User,
    Assistant,
    Tool,
}

impl MessageRole {
    pub fn as_str(&self) -> &'static str {
        match self {
            MessageRole::System => "system",
            MessageRole::User => "user",
            MessageRole::Assistant => "assistant",
            MessageRole::Tool => "tool",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    pub id: String,
    pub name: String,
    pub arguments: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatOptions {
    pub temperature: Option<f32>,
    pub max_tokens: Option<usize>,
    pub top_p: Option<f32>,
    pub thinking: Option<ThinkingOptions>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<ToolDefinition>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThinkingOptions {
    pub enabled: bool,
    pub effort: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDefinition {
    pub name: String,
    pub description: String,
    pub parameters: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatResponse {
    pub id: String,
    pub model: String,
    pub choices: Vec<ChatChoice>,
    pub usage: Option<Usage>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatChoice {
    pub message: Message,
    pub finish_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Usage {
    pub prompt_tokens: usize,
    pub completion_tokens: usize,
    pub total_tokens: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelInfo {
    pub id: String,
    pub provider: String,
    pub supports_tools: bool,
    pub supports_reasoning: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamChunk {
    pub delta: StreamDelta,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StreamDelta {
    Thinking(String),
    Content(String),
    ToolCallStart {
        id: String,
        name: String,
        arguments: String,
    },
    ToolCallDelta {
        id: String,
        arguments_delta: String,
    },
    Done,
}
