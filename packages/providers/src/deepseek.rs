use super::provider::{ChatOptions, ChatResponse, LLMProvider, Message, ModelInfo};
use anyhow::Result;
use async_trait::async_trait;

const DEEPSEEK_BASE_URL: &str = "https://api.deepseek.com/v1";

pub struct DeepSeekProvider {
    api_key: String,
    base_url: String,
    client: reqwest::Client,
}

impl DeepSeekProvider {
    pub fn new(api_key: String) -> Self {
        Self {
            api_key,
            base_url: DEEPSEEK_BASE_URL.to_string(),
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
}

#[async_trait]
impl LLMProvider for DeepSeekProvider {
    async fn chat_completions(
        &self,
        messages: Vec<Message>,
        model: &str,
        options: ChatOptions,
    ) -> Result<ChatResponse> {
        let url = format!("{}/chat/completions", self.base_url);

        let mut request_body = serde_json::json!({
            "model": model,
            "messages": messages,
        });

        if let Some(temp) = options.temperature {
            request_body["temperature"] = serde_json::json!(temp);
        }
        if let Some(max_tokens) = options.max_tokens {
            request_body["max_tokens"] = serde_json::json!(max_tokens);
        }
        if let Some(top_p) = options.top_p {
            request_body["top_p"] = serde_json::json!(top_p);
        }

        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&request_body)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            anyhow::bail!("DeepSeek API request failed: {} - {}", status, body);
        }

        #[derive(serde::Deserialize)]
        struct DeepSeekResponse {
            id: String,
            model: String,
            choices: Vec<DeepSeekChoice>,
            usage: Option<DeepSeekUsage>,
        }

        #[derive(serde::Deserialize)]
        struct DeepSeekChoice {
            message: DeepSeekMessage,
            finish_reason: Option<String>,
        }

        #[derive(serde::Deserialize)]
        struct DeepSeekMessage {
            role: String,
            content: Option<String>,
        }

        #[derive(serde::Deserialize)]
        struct DeepSeekUsage {
            prompt_tokens: usize,
            completion_tokens: usize,
            total_tokens: usize,
        }

        let ds_response: DeepSeekResponse = response.json().await?;

        Ok(ChatResponse {
            id: ds_response.id,
            model: ds_response.model,
            choices: ds_response
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
            usage: ds_response.usage.map(|u| super::provider::Usage {
                prompt_tokens: u.prompt_tokens,
                completion_tokens: u.completion_tokens,
                total_tokens: u.total_tokens,
            }),
        })
    }

    async fn list_models(&self) -> Result<Vec<ModelInfo>> {
        Ok(vec![
            ModelInfo {
                id: "deepseek-v4-pro".to_string(),
                provider: "deepseek".to_string(),
                supports_tools: true,
                supports_reasoning: true,
            },
            ModelInfo {
                id: "deepseek-v4-flash".to_string(),
                provider: "deepseek".to_string(),
                supports_tools: true,
                supports_reasoning: true,
            },
        ])
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

impl Default for DeepSeekProvider {
    fn default() -> Self {
        Self::new(std::env::var("DEEPSEEK_API_KEY").unwrap_or_default())
    }
}
