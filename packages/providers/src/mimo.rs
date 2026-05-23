use super::provider::{
    ChatOptions, ChatResponse, LLMProvider, Message, ModelInfo, StreamChunk, StreamDelta,
};
use anyhow::Result;
use async_trait::async_trait;
use futures_util::StreamExt;
use serde::Deserialize;
use tokio::sync::mpsc;

const MIMO_BASE_URL: &str = "https://api.xiaomimimo.com/v1";
const MIMO_TOKEN_PLAN_BASE_URL: &str = "https://token-plan-cn.xiaomimimo.com/v1";
#[allow(dead_code)]
const DEFAULT_MODEL: &str = "mimo-v2.5-pro";

pub struct MimoProvider {
    api_key: String,
    base_url: String,
    client: reqwest::Client,
}

#[derive(Debug, Deserialize)]
struct MimoResponse {
    id: String,
    model: String,
    choices: Vec<MimoChoice>,
    usage: Option<MimoUsage>,
}

#[derive(Debug, Deserialize)]
struct MimoChoice {
    message: MimoMessage,
    finish_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
struct MimoMessage {
    role: String,
    content: Option<String>,
    #[allow(dead_code)]
    reasoning_content: Option<String>,
}

#[derive(Debug, Deserialize)]
struct MimoUsage {
    prompt_tokens: usize,
    completion_tokens: usize,
    total_tokens: usize,
}

#[derive(Debug, Deserialize)]
struct MimoStreamChunk {
    choices: Vec<MimoStreamChoice>,
}

#[derive(Debug, Deserialize)]
struct MimoStreamChoice {
    delta: MimoStreamDelta,
    finish_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
struct MimoStreamDelta {
    content: Option<String>,
    reasoning_content: Option<String>,
    tool_calls: Option<Vec<MimoStreamToolCall>>,
}

#[derive(Debug, Deserialize)]
struct MimoStreamToolCall {
    id: Option<String>,
    index: Option<usize>,
    function: Option<MimoStreamFunction>,
}

#[derive(Debug, Deserialize)]
struct MimoStreamFunction {
    name: Option<String>,
    arguments: Option<String>,
}

impl MimoProvider {
    pub fn new(api_key: String) -> Self {
        let base_url = if api_key.starts_with("tp-") {
            MIMO_TOKEN_PLAN_BASE_URL
        } else {
            MIMO_BASE_URL
        };
        Self {
            api_key,
            base_url: base_url.to_string(),
            client: reqwest::Client::new(),
        }
    }

    pub fn with_base_url(api_key: String, base_url: String) -> Self {
        Self {
            api_key,
            base_url,
            client: reqwest::Client::new(),
        }
    }

    fn fallback_models(&self) -> Vec<ModelInfo> {
        vec![
            ModelInfo {
                id: "mimo-v2.5-pro".to_string(),
                provider: "mimo".to_string(),
                supports_tools: true,
                supports_reasoning: true,
            },
            ModelInfo {
                id: "mimo-v2.5-flash".to_string(),
                provider: "mimo".to_string(),
                supports_tools: true,
                supports_reasoning: true,
            },
        ]
    }

    fn build_request(
        &self,
        messages: Vec<Message>,
        model: &str,
        options: ChatOptions,
    ) -> serde_json::Value {
        let mut request_body = serde_json::json!({
            "model": model,
            "messages": messages.iter().map(|m| {
                let mut msg = serde_json::json!({
                    "role": m.role.as_str(),
                    "content": m.content,
                });
                if let Some(name) = &m.name {
                    msg["name"] = serde_json::json!(name);
                }
                msg
            }).collect::<Vec<_>>(),
        });

        if let Some(temp) = options.temperature {
            request_body["temperature"] = serde_json::json!(temp);
        }

        if let Some(max_tokens) = options.max_tokens {
            request_body["max_completion_tokens"] = serde_json::json!(max_tokens);
        }

        if let Some(top_p) = options.top_p {
            request_body["top_p"] = serde_json::json!(top_p);
        }

        if let Some(thinking) = &options.thinking {
            if thinking.enabled {
                let mut thinking_obj = serde_json::json!({
                    "type": "enabled"
                });
                if let Some(effort) = &thinking.effort {
                    thinking_obj["effort"] = serde_json::json!(effort);
                }
                request_body["thinking"] = thinking_obj;
            }
        }

        if let Some(tools) = &options.tools {
            request_body["tools"] = serde_json::json!(
                tools
                    .iter()
                    .map(|t| {
                        serde_json::json!({
                            "type": "function",
                            "function": {
                                "name": t.name,
                                "description": t.description,
                                "parameters": t.parameters,
                            }
                        })
                    })
                    .collect::<Vec<_>>()
            );
        }

        request_body
    }

    fn parse_response(&self, response: MimoResponse) -> ChatResponse {
        ChatResponse {
            id: response.id,
            model: response.model,
            choices: response
                .choices
                .into_iter()
                .map(|c| super::provider::ChatChoice {
                    message: Message {
                        role: match c.message.role.as_str() {
                            "system" => super::provider::MessageRole::System,
                            "user" => super::provider::MessageRole::User,
                            "assistant" => super::provider::MessageRole::Assistant,
                            "tool" => super::provider::MessageRole::Tool,
                            _ => super::provider::MessageRole::User,
                        },
                        content: c.message.content.unwrap_or_default(),
                        name: None,
                        tool_calls: None,
                    },
                    finish_reason: c.finish_reason,
                })
                .collect(),
            usage: response.usage.map(|u| super::provider::Usage {
                prompt_tokens: u.prompt_tokens,
                completion_tokens: u.completion_tokens,
                total_tokens: u.total_tokens,
            }),
        }
    }
}

#[async_trait]
impl LLMProvider for MimoProvider {
    async fn chat_completions(
        &self,
        messages: Vec<Message>,
        model: &str,
        options: ChatOptions,
    ) -> Result<ChatResponse> {
        let url = format!("{}/chat/completions", self.base_url);
        let request_body = self.build_request(messages, model, options);

        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&request_body)
            .send()
            .await?;

        if response.status() == 401 {
            anyhow::bail!("Authentication failed: Invalid API key");
        }

        if response.status() == 429 {
            anyhow::bail!("Rate limit exceeded");
        }

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            anyhow::bail!("API request failed with status {}: {}", status, body);
        }

        let mimo_response: MimoResponse = response.json().await?;
        Ok(self.parse_response(mimo_response))
    }

    async fn chat_completions_stream(
        &self,
        messages: Vec<Message>,
        model: &str,
        options: ChatOptions,
    ) -> Result<mpsc::UnboundedReceiver<anyhow::Result<StreamChunk>>> {
        let url = format!("{}/chat/completions", self.base_url);
        let mut request_body = self.build_request(messages, model, options);
        request_body["stream"] = serde_json::json!(true);

        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .header("Accept", "text/event-stream")
            .json(&request_body)
            .send()
            .await?;

        if response.status() == 401 {
            anyhow::bail!("Authentication failed: Invalid API key");
        }
        if response.status() == 429 {
            anyhow::bail!("Rate limit exceeded");
        }
        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            anyhow::bail!("API request failed with status {}: {}", status, body);
        }

        let (tx, rx) = mpsc::unbounded_channel();

        let mut stream = response.bytes_stream();
        tokio::spawn(async move {
            let mut buffer = String::new();
            let mut tool_call_buffers: std::collections::HashMap<usize, (Option<String>, String)> =
                std::collections::HashMap::new();

            while let Some(chunk_result) = stream.next().await {
                match chunk_result {
                    Ok(bytes) => {
                        let text = String::from_utf8_lossy(&bytes);
                        buffer.push_str(&text);

                        while let Some(line_end) = buffer.find('\n') {
                            let line = buffer[..line_end].trim().to_string();
                            buffer = buffer[line_end + 1..].to_string();

                            if line.is_empty() || line.starts_with(':') {
                                continue;
                            }

                            let data = if let Some(d) = line.strip_prefix("data: ") {
                                d
                            } else if let Some(d) = line.strip_prefix("data:") {
                                d
                            } else {
                                continue;
                            };

                            if data == "[DONE]" {
                                for (_, (name, args)) in tool_call_buffers.drain() {
                                    if let Some(n) = name {
                                        let _ = tx.send(Ok(StreamChunk {
                                            delta: StreamDelta::ToolCallStart {
                                                id: format!("call_{}", n),
                                                name: n,
                                                arguments: args,
                                            },
                                        }));
                                    }
                                }
                                let _ = tx.send(Ok(StreamChunk {
                                    delta: StreamDelta::Done,
                                }));
                                return;
                            }

                            match serde_json::from_str::<MimoStreamChunk>(data) {
                                Ok(chunk) => {
                                    for choice in chunk.choices {
                                        if let Some(ref finish_reason) = choice.finish_reason {
                                            if finish_reason == "tool_calls"
                                                || finish_reason == "stop"
                                            {
                                                for (idx, (name, args)) in tool_call_buffers.drain()
                                                {
                                                    if let Some(n) = name {
                                                        let _ = tx.send(Ok(StreamChunk {
                                                            delta: StreamDelta::ToolCallStart {
                                                                id: format!("call_{}_{}", n, idx),
                                                                name: n,
                                                                arguments: args,
                                                            },
                                                        }));
                                                    }
                                                }
                                                if finish_reason == "stop" {
                                                    let _ = tx.send(Ok(StreamChunk {
                                                        delta: StreamDelta::Done,
                                                    }));
                                                    return;
                                                }
                                            }
                                        }

                                        if let Some(ref reasoning) = choice.delta.reasoning_content
                                        {
                                            let _ = tx.send(Ok(StreamChunk {
                                                delta: StreamDelta::Thinking(reasoning.clone()),
                                            }));
                                        }

                                        if let Some(ref content) = choice.delta.content {
                                            let _ = tx.send(Ok(StreamChunk {
                                                delta: StreamDelta::Content(content.clone()),
                                            }));
                                        }

                                        if let Some(ref tool_calls) = choice.delta.tool_calls {
                                            for tc in tool_calls {
                                                let idx = tc.index.unwrap_or(0);
                                                let entry = tool_call_buffers
                                                    .entry(idx)
                                                    .or_insert_with(|| (None, String::new()));
                                                if let Some(ref id) = tc.id {
                                                    entry.0 = Some(id.clone());
                                                }
                                                if let Some(ref func) = tc.function {
                                                    if let Some(ref name) = func.name {
                                                        entry.0 = Some(name.clone());
                                                    }
                                                    if let Some(ref args) = func.arguments {
                                                        entry.1.push_str(args);
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                                Err(_) => {}
                            }
                        }
                    }
                    Err(_) => {
                        let _ = tx.send(Err(anyhow::anyhow!("Stream connection error")));
                        return;
                    }
                }
            }

            let _ = tx.send(Ok(StreamChunk {
                delta: StreamDelta::Done,
            }));
        });

        Ok(rx)
    }

    async fn list_models(&self) -> Result<Vec<ModelInfo>> {
        let url = format!("{}/models", self.base_url);
        let response = self
            .client
            .get(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .send()
            .await?;

        if !response.status().is_success() {
            return Ok(self.fallback_models());
        }

        #[derive(Debug, Deserialize)]
        struct ModelsResponse {
            data: Vec<ModelData>,
        }

        #[derive(Debug, Deserialize)]
        struct ModelData {
            id: String,
            owned_by: Option<String>,
        }

        match response.json::<ModelsResponse>().await {
            Ok(models_resp) => {
                let models: Vec<ModelInfo> = models_resp
                    .data
                    .into_iter()
                    .map(|m| {
                        let id = m.id.clone();
                        let supports_reasoning =
                            id.contains("mimo") || id.contains("reasoning") || id.contains("think");
                        let supports_tools =
                            id.contains("mimo") || id.contains("gpt") || id.contains("claude");
                        ModelInfo {
                            id,
                            provider: m.owned_by.unwrap_or_else(|| "mimo".to_string()),
                            supports_tools,
                            supports_reasoning,
                        }
                    })
                    .collect();
                if models.is_empty() {
                    Ok(self.fallback_models())
                } else {
                    Ok(models)
                }
            }
            Err(_) => Ok(self.fallback_models()),
        }
    }

    async fn health_check(&self) -> bool {
        let url = format!("{}/models", self.base_url);
        match self
            .client
            .get(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .send()
            .await
        {
            Ok(response) => response.status().is_success(),
            Err(_) => false,
        }
    }
}

impl Default for MimoProvider {
    fn default() -> Self {
        Self::new(std::env::var("MIMO_API_KEY").unwrap_or_default())
    }
}
