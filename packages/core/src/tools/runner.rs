use super::types::{ToolCall, ToolDescriptor, ToolError, ToolHandler, ToolResult};
use crate::memory::MemoryManager;
use crate::memory::memory::MemoryTarget;
use crate::memory::recall::RecallArchive;
use std::path::Path;
use std::process::Command;

pub struct ToolRunner {
    memory_manager: MemoryManager,
    recall_archive: Option<RecallArchive>,
}

impl ToolRunner {
    pub fn new(memory_manager: MemoryManager, recall_archive: Option<RecallArchive>) -> Self {
        Self {
            memory_manager,
            recall_archive,
        }
    }

    pub fn memory_manager(&self) -> &MemoryManager {
        &self.memory_manager
    }

    pub async fn execute(&self, call: ToolCall, descriptor: &ToolDescriptor) -> ToolResult {
        match &descriptor.handler {
            ToolHandler::FileRead => self.execute_file_read(&call).await,
            ToolHandler::FileWrite => self.execute_file_write(&call).await,
            ToolHandler::Shell => self.execute_shell(&call).await,
            ToolHandler::Grep => self.execute_grep(&call).await,
            ToolHandler::Glob => self.execute_glob(&call).await,
            ToolHandler::Git => self.execute_git(&call).await,
            ToolHandler::WebFetch => self.execute_web_fetch(&call).await,
            ToolHandler::WebSearch => self.execute_web_search(&call).await,
            ToolHandler::Remember => self.execute_remember(&call).await,
            ToolHandler::MemoryReplace => self.execute_memory_replace(&call).await,
            ToolHandler::MemoryRemove => self.execute_memory_remove(&call).await,
            ToolHandler::RecallArchive => self.execute_recall_archive(&call).await,
            ToolHandler::AgentSpawn => self.execute_agent_spawn(&call).await,
            ToolHandler::AgentWait => self.execute_agent_wait(&call).await,
            ToolHandler::AgentResult => self.execute_agent_result(&call).await,
            ToolHandler::AgentCancel => self.execute_agent_cancel(&call).await,
            ToolHandler::AgentList => self.execute_agent_list(&call).await,
        }
    }

    async fn execute_agent_spawn(&self, _call: &ToolCall) -> ToolResult {
        ToolResult::Error(ToolError::new(
            "agent_spawn_error",
            "Agent spawn must be handled by the TUI runtime".to_string(),
        ))
    }

    async fn execute_agent_wait(&self, _call: &ToolCall) -> ToolResult {
        ToolResult::Error(ToolError::new(
            "agent_wait_error",
            "Agent wait must be handled by the TUI runtime".to_string(),
        ))
    }

    async fn execute_agent_result(&self, _call: &ToolCall) -> ToolResult {
        ToolResult::Error(ToolError::new(
            "agent_result_error",
            "Agent result must be handled by the TUI runtime".to_string(),
        ))
    }

    async fn execute_agent_cancel(&self, _call: &ToolCall) -> ToolResult {
        ToolResult::Error(ToolError::new(
            "agent_cancel_error",
            "Agent cancel must be handled by the TUI runtime".to_string(),
        ))
    }

    async fn execute_agent_list(&self, _call: &ToolCall) -> ToolResult {
        ToolResult::Error(ToolError::new(
            "agent_list_error",
            "Agent list must be handled by the TUI runtime".to_string(),
        ))
    }

    async fn execute_file_read(&self, call: &ToolCall) -> ToolResult {
        let path = call
            .arguments
            .get("path")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        if path.is_empty() {
            return ToolResult::Error(ToolError::new(
                "file_read_error",
                "Path is required".to_string(),
            ));
        }

        match std::fs::read_to_string(path) {
            Ok(content) => ToolResult::Success(content),
            Err(e) => ToolResult::Error(ToolError::new("file_read_error", e.to_string())),
        }
    }

    async fn execute_file_write(&self, call: &ToolCall) -> ToolResult {
        let path = call
            .arguments
            .get("path")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let content = call
            .arguments
            .get("content")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        if path.is_empty() {
            return ToolResult::Error(ToolError::new(
                "file_write_error",
                "Path is required".to_string(),
            ));
        }

        match std::fs::write(path, content) {
            Ok(_) => ToolResult::Success(format!("Successfully written to {}", path)),
            Err(e) => ToolResult::Error(ToolError::new("file_write_error", e.to_string())),
        }
    }

    async fn execute_shell(&self, call: &ToolCall) -> ToolResult {
        let cmd = call
            .arguments
            .get("command")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        if cmd.is_empty() {
            return ToolResult::Error(ToolError::new(
                "shell_error",
                "Command is required".to_string(),
            ));
        }

        let output = Command::new("sh").arg("-c").arg(cmd).output();

        match output {
            Ok(output) => {
                let stdout = String::from_utf8_lossy(&output.stdout);
                let stderr = String::from_utf8_lossy(&output.stderr);
                if output.status.success() {
                    if stdout.is_empty() {
                        ToolResult::Success("(no output)".to_string())
                    } else {
                        ToolResult::Success(stdout.to_string())
                    }
                } else {
                    if stderr.is_empty() {
                        ToolResult::Error(ToolError::new("shell_error", stdout.to_string()))
                    } else {
                        ToolResult::Error(ToolError::new("shell_error", stderr.to_string()))
                    }
                }
            }
            Err(e) => ToolResult::Error(ToolError::new("shell_error", e.to_string())),
        }
    }

    async fn execute_grep(&self, call: &ToolCall) -> ToolResult {
        let pattern = call
            .arguments
            .get("pattern")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let path = call
            .arguments
            .get("path")
            .and_then(|v| v.as_str())
            .unwrap_or(".");
        let _show_line_numbers = call
            .arguments
            .get("-n")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        let case_insensitive = call
            .arguments
            .get("-i")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        if pattern.is_empty() {
            return ToolResult::Error(ToolError::new(
                "grep_error",
                "Pattern is required".to_string(),
            ));
        }

        let mut cmd = Command::new("grep");
        cmd.arg("-n".to_string())
            .arg("--".to_string())
            .arg(pattern.to_string())
            .arg(path);

        if case_insensitive {
            cmd.arg("-i");
        }

        match cmd.output() {
            Ok(output) => {
                let stdout = String::from_utf8_lossy(&output.stdout);
                let stderr = String::from_utf8_lossy(&output.stderr);
                if output.status.success() {
                    ToolResult::Success(stdout.to_string())
                } else if stdout.is_empty() {
                    ToolResult::Success("(no matches found)".to_string())
                } else {
                    ToolResult::Error(ToolError::new("grep_error", stderr.to_string()))
                }
            }
            Err(e) => ToolResult::Error(ToolError::new("grep_error", e.to_string())),
        }
    }

    async fn execute_glob(&self, call: &ToolCall) -> ToolResult {
        let pattern = call
            .arguments
            .get("pattern")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let path = call
            .arguments
            .get("path")
            .and_then(|v| v.as_str())
            .unwrap_or(".");

        if pattern.is_empty() {
            return ToolResult::Error(ToolError::new(
                "glob_error",
                "Pattern is required".to_string(),
            ));
        }

        let output = Command::new("find")
            .arg(path)
            .arg("-type")
            .arg("f")
            .arg("-name")
            .arg(pattern)
            .output();

        match output {
            Ok(output) => {
                let stdout = String::from_utf8_lossy(&output.stdout);
                if stdout.is_empty() {
                    ToolResult::Success("(no matches found)".to_string())
                } else {
                    ToolResult::Success(stdout.to_string())
                }
            }
            Err(e) => ToolResult::Error(ToolError::new("glob_error", e.to_string())),
        }
    }

    async fn execute_git(&self, call: &ToolCall) -> ToolResult {
        let path = call
            .arguments
            .get("path")
            .and_then(|v| v.as_str())
            .unwrap_or(".");

        let output = Command::new("git")
            .args(&["-C", path, "status", "--porcelain"])
            .output();

        match output {
            Ok(output) => {
                let stdout = String::from_utf8_lossy(&output.stdout);
                let stderr = String::from_utf8_lossy(&output.stderr);
                if output.status.success() {
                    if stdout.is_empty() {
                        ToolResult::Success("(clean)".to_string())
                    } else {
                        ToolResult::Success(stdout.to_string())
                    }
                } else {
                    ToolResult::Error(ToolError::new("git_error", stderr.to_string()))
                }
            }
            Err(e) => ToolResult::Error(ToolError::new("git_error", e.to_string())),
        }
    }

    async fn execute_web_fetch(&self, call: &ToolCall) -> ToolResult {
        let url = call
            .arguments
            .get("url")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        if url.is_empty() {
            return ToolResult::Error(ToolError::new(
                "web_fetch_error",
                "URL is required".to_string(),
            ));
        }

        match reqwest::get(url).await {
            Ok(response) => match response.text().await {
                Ok(text) => ToolResult::Success(text),
                Err(e) => ToolResult::Error(ToolError::new("web_fetch_error", e.to_string())),
            },
            Err(e) => ToolResult::Error(ToolError::new("web_fetch_error", e.to_string())),
        }
    }

    async fn execute_web_search(&self, call: &ToolCall) -> ToolResult {
        let query = call
            .arguments
            .get("query")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        if query.is_empty() {
            return ToolResult::Error(ToolError::new(
                "web_search_error",
                "Query is required".to_string(),
            ));
        }

        let encoded_query = form_urlencoded::byte_serialize(query.as_bytes()).collect::<String>();
        let search_url = format!("https://duckduckgo.com/html/?q={}", encoded_query);

        match reqwest::get(&search_url).await {
            Ok(response) => match response.text().await {
                Ok(text) => {
                    let results = extract_duckduckgo_results(&text);
                    ToolResult::Success(results)
                }
                Err(e) => ToolResult::Error(ToolError::new("web_search_error", e.to_string())),
            },
            Err(e) => ToolResult::Error(ToolError::new("web_search_error", e.to_string())),
        }
    }

    async fn execute_remember(&self, call: &ToolCall) -> ToolResult {
        let note = call
            .arguments
            .get("note")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        if note.is_empty() {
            return ToolResult::Error(ToolError::new(
                "remember_error",
                "Note is required".to_string(),
            ));
        }

        let target_str = call
            .arguments
            .get("target")
            .and_then(|v| v.as_str())
            .unwrap_or("memory");
        let target = match target_str {
            "user" => MemoryTarget::User,
            _ => MemoryTarget::Memory,
        };

        let result = self.memory_manager.add(target, note);
        if result.ok {
            ToolResult::Success(result.message)
        } else {
            ToolResult::Error(ToolError::new("remember_error", result.message))
        }
    }

    async fn execute_recall_archive(&self, call: &ToolCall) -> ToolResult {
        let query = call
            .arguments
            .get("query")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        if query.is_empty() {
            return ToolResult::Error(ToolError::new(
                "recall_error",
                "Query is required".to_string(),
            ));
        }

        let max_results = call
            .arguments
            .get("max_results")
            .and_then(|v| v.as_u64())
            .unwrap_or(3)
            .min(10) as usize;

        let session_id = call
            .arguments
            .get("session_id")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        let session_content = call
            .arguments
            .get("session_content")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        match &self.recall_archive {
            Some(archive) => {
                let hits = archive.search(query, session_content, max_results, session_id);

                if hits.is_empty() {
                    ToolResult::Success("(no matching memories found)".to_string())
                } else {
                    let output: String = hits
                        .iter()
                        .map(|hit| {
                            format!(
                                "[Session: {}] (score: {:.2})\n{}\n---\n",
                                hit.session_id, hit.score, hit.excerpt
                            )
                        })
                        .collect();
                    ToolResult::Success(output)
                }
            }
            None => ToolResult::Error(ToolError::new(
                "recall_error",
                "Recall archive not initialized".to_string(),
            )),
        }
    }

    async fn execute_memory_replace(&self, call: &ToolCall) -> ToolResult {
        let target_str = call
            .arguments
            .get("target")
            .and_then(|v| v.as_str())
            .unwrap_or("memory");
        let target = match target_str {
            "user" => MemoryTarget::User,
            _ => MemoryTarget::Memory,
        };

        let old_text = call
            .arguments
            .get("old_text")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let content = call
            .arguments
            .get("content")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        if old_text.is_empty() {
            return ToolResult::Error(ToolError::new(
                "memory_replace_error",
                "old_text is required".to_string(),
            ));
        }
        if content.is_empty() {
            return ToolResult::Error(ToolError::new(
                "memory_replace_error",
                "content is required".to_string(),
            ));
        }

        let result = self.memory_manager.replace(target, old_text, content);
        if result.ok {
            ToolResult::Success(result.message)
        } else {
            ToolResult::Error(ToolError::new("memory_replace_error", result.message))
        }
    }

    async fn execute_memory_remove(&self, call: &ToolCall) -> ToolResult {
        let target_str = call
            .arguments
            .get("target")
            .and_then(|v| v.as_str())
            .unwrap_or("memory");
        let target = match target_str {
            "user" => MemoryTarget::User,
            _ => MemoryTarget::Memory,
        };

        let old_text = call
            .arguments
            .get("old_text")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        if old_text.is_empty() {
            return ToolResult::Error(ToolError::new(
                "memory_remove_error",
                "old_text is required".to_string(),
            ));
        }

        let result = self.memory_manager.remove(target, old_text);
        if result.ok {
            ToolResult::Success(result.message)
        } else {
            ToolResult::Error(ToolError::new("memory_remove_error", result.message))
        }
    }
}

fn extract_duckduckgo_results(html: &str) -> String {
    let mut results = Vec::new();
    for line in html.lines() {
        if line.contains("result__snippet") {
            if let Some(snippet) = extract_snippet(line) {
                if !snippet.is_empty() {
                    results.push(snippet);
                }
            }
        }
    }
    if results.is_empty() {
        "(no results found)".to_string()
    } else {
        results.join("\n---\n")
    }
}

fn extract_snippet(line: &str) -> Option<String> {
    let parts: Vec<&str> = line.split("result__snippet").collect();
    if parts.len() < 2 {
        return None;
    }
    let after_tag = parts[1];
    let start = after_tag.find('>')?;
    let content = &after_tag[start + 1..];
    let end = content.find('<')?;
    Some(content[..end].trim().to_string())
}

impl Default for ToolRunner {
    fn default() -> Self {
        Self::new(MemoryManager::new(Path::new("/tmp/.mimo")), None)
    }
}
