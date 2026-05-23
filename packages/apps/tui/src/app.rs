use anyhow::Result;
use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers, MouseEventKind};
use crossterm::{event, execute, terminal};
use dirs;
use mimo_core::{
    MemoryManager, MemoryTarget, RecallArchive, SessionManager, SubAgentManager, SubAgentResult,
    SubAgentRole, ToolRegistry,
    session::{MessageRole, Session},
    tools::{ToolCall as CoreToolCall, ToolError, ToolResult as CoreToolResult, ToolRunner},
};
use mimo_providers::{
    LLMProvider, MimoProvider,
    provider::{
        ChatOptions, Message as ProviderMessage, MessageRole as ProviderMessageRole, StreamDelta,
        ThinkingOptions, ToolDefinition,
    },
};
use ratatui::{Terminal, backend::CrosstermBackend};
use std::collections::HashMap;
use std::io;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::mpsc;

use crate::components::{
    ApprovalRequest, Command, FooterComponent, HeaderComponent, InputAction, InputComponent,
    MemorySubCommand, MessagesComponent, SidebarPanel, get_command_completions, is_quick_memory,
    messages::ToolCallStatus, parse_command, render_memory_popup, render_tools_popup,
};
use crate::theme::Theme;
use crate::views::{AgentsView, ChatView, HelpView, HistoryView};

const DEFAULT_MODEL: &str = "mimo-v2.5-pro";
const STATUS_TIMEOUT_MS: u64 = 4000;

fn is_ctrl_or_cmd(mods: KeyModifiers) -> bool {
    mods.contains(KeyModifiers::CONTROL)
}

fn get_api_key() -> Result<String> {
    if let Ok(key) = std::env::var("MIMO_API_KEY") {
        if !key.is_empty() {
            return Ok(key);
        }
    }

    if let Ok(key) = std::env::var("MIMO_API_KEY_FILE") {
        if let Ok(content) = std::fs::read_to_string(&key) {
            let trimmed = content.trim().to_string();
            if !trimmed.is_empty() {
                return Ok(trimmed);
            }
        }
    }

    let home = dirs::home_dir().unwrap_or_else(|| std::path::PathBuf::from("."));
    let api_key_file = home.join(".mimo").join("api_key");
    if let Ok(content) = std::fs::read_to_string(&api_key_file) {
        let trimmed = content.trim().to_string();
        if !trimmed.is_empty() {
            return Ok(trimmed);
        }
    }

    if let Ok(config) = mimo_config::Config::load() {
        if let Some(key) = config.api_key {
            if !key.is_empty() {
                return Ok(key);
            }
        }
    }

    Err(anyhow::anyhow!(
        "MIMO_API_KEY not set. Set it via env var, ~/.mimo/api_key, or ~/.mimo/config.toml"
    ))
}

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AppMode {
    Normal,
    Insert,
    Command,
    HelpPopup,
    ToolsPopup,
    MemoryPopup,
    ApprovalPending,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ActiveView {
    Chat,
    SubAgents,
    History,
}

#[derive(Debug, Clone)]
pub enum StatusType {
    Info,
    Success,
    Warning,
    Error,
}

#[derive(Debug, Clone)]
pub struct StatusMessage {
    pub message: String,
    pub status_type: StatusType,
    pub created_at: Instant,
}

#[allow(dead_code)]
pub enum StreamEvent {
    ThinkingDelta(String),
    ContentDelta(String),
    ToolCallStart {
        id: String,
        name: String,
        arguments: String,
    },
    ToolCallResult {
        id: String,
        result: String,
        success: bool,
    },
    AgentProgress {
        agent_id: String,
        status: String,
        progress: u8,
        message: Option<String>,
    },
    Complete,
    Error(String),
}

pub struct App {
    pub should_quit: bool,
    pub mode: AppMode,
    pub active_view: ActiveView,
    pub status_message: Option<StatusMessage>,
    pub model: String,
    pub memory_enabled: bool,
    pub theme: Theme,

    pub input: InputComponent,
    pub messages: MessagesComponent,
    pub chat_view: ChatView,
    pub help_view: HelpView,
    pub agents_view: AgentsView,
    pub history_view: HistoryView,

    pub session_manager: Option<SessionManager>,
    pub tool_registry: Option<Arc<ToolRegistry>>,
    pub memory_manager: Option<MemoryManager>,
    pub tool_runner: Option<Arc<ToolRunner>>,
    pub subagent_manager: Option<Arc<SubAgentManager>>,
    pub provider: Option<Arc<MimoProvider>>,
    pub current_session: Option<Session>,
    pub storage_path: Option<PathBuf>,

    pub is_streaming: bool,
    pub stream_msg_id: Option<String>,
    pub stream_rx: Option<mpsc::UnboundedReceiver<StreamEvent>>,
    pub stream_cancel: Option<tokio::sync::watch::Sender<bool>>,

    pub approval_request: Option<ApprovalRequest>,
    pub command_buffer: String,
    pub completion_candidates: Vec<String>,
    pub completion_index: usize,
    pub model_list: Vec<mimo_providers::provider::ModelInfo>,
    pub scroll_amount: usize,

    cached_agents: Vec<mimo_core::SubAgent>,
    cached_agent_count: usize,
    cached_sessions: Vec<mimo_core::Session>,
    last_data_refresh: Instant,

    total_input_tokens: usize,
    total_output_tokens: usize,
    total_tool_calls: usize,
    turn_count: usize,
    session_start: Instant,
    thinking_active: bool,
    models_fetch_needed: bool,
}

impl App {
    pub fn new() -> Self {
        Self {
            should_quit: false,
            mode: AppMode::Normal,
            active_view: ActiveView::Chat,
            status_message: None,
            model: DEFAULT_MODEL.to_string(),
            memory_enabled: false,
            theme: Theme::default(),
            input: InputComponent::new(),
            messages: MessagesComponent::new(),
            chat_view: ChatView::new(),
            help_view: HelpView::new(),
            agents_view: AgentsView::new(),
            history_view: HistoryView::new(),
            session_manager: None,
            tool_registry: None,
            memory_manager: None,
            tool_runner: None,
            subagent_manager: None,
            provider: None,
            current_session: None,
            storage_path: None,
            is_streaming: false,
            stream_msg_id: None,
            stream_rx: None,
            stream_cancel: None,
            approval_request: None,
            command_buffer: String::new(),
            completion_candidates: Vec::new(),
            completion_index: 0,
            model_list: Vec::new(),
            scroll_amount: 5,
            cached_agents: Vec::new(),
            cached_agent_count: 0,
            cached_sessions: Vec::new(),
            last_data_refresh: Instant::now(),
            total_input_tokens: 0,
            total_output_tokens: 0,
            total_tool_calls: 0,
            turn_count: 0,
            session_start: Instant::now(),
            thinking_active: false,
            models_fetch_needed: false,
        }
    }

    pub fn initialize(&mut self) -> Result<()> {
        let api_key = get_api_key()?;

        self.memory_enabled = std::env::var("MIMO_MEMORY").is_ok();
        if let Ok(model) = std::env::var("MIMO_MODEL") {
            self.model = model;
        }

        if let Ok(config) = mimo_config::Config::load() {
            self.theme = Theme::from_config(&config.theme);
        }

        let storage_path = dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".mimo");
        std::fs::create_dir_all(&storage_path.join("sessions"))?;
        self.storage_path = Some(storage_path.clone());

        let provider = Arc::new(MimoProvider::new(api_key));

        let tool_registry = Arc::new(ToolRegistry::new());
        let mut memory_manager = MemoryManager::new(&storage_path);
        memory_manager.take_snapshot();
        let recall_archive = Some(RecallArchive::new(storage_path.join("sessions")));
        let tool_runner = Arc::new(ToolRunner::new(
            MemoryManager::new(&storage_path),
            recall_archive,
        ));
        let subagent_manager = Arc::new(SubAgentManager::new());
        let session_manager = SessionManager::new(storage_path.clone());

        self.provider = Some(provider.clone());
        self.tool_registry = Some(tool_registry);
        self.memory_manager = Some(memory_manager);
        self.tool_runner = Some(tool_runner);
        self.subagent_manager = Some(subagent_manager);
        self.session_manager = Some(session_manager);

        self.fetch_models();

        self.set_status("Initialized successfully", StatusType::Success);
        Ok(())
    }

    pub fn fetch_models(&mut self) {
        self.models_fetch_needed = true;
    }

    pub async fn fetch_models_async(&mut self) {
        if let Some(ref provider) = self.provider {
            match provider.list_models().await {
                Ok(models) => {
                    self.model_list = models;
                    if !self.model_list.iter().any(|m| m.id == self.model) {
                        if let Some(first) = self.model_list.first() {
                            self.model = first.id.clone();
                        }
                    }
                    self.set_status(
                        &format!("Loaded {} models", self.model_list.len()),
                        StatusType::Success,
                    );
                }
                Err(_) => {
                    self.model_list = vec![
                        mimo_providers::provider::ModelInfo {
                            id: "mimo-v2.5-pro".to_string(),
                            provider: "mimo".to_string(),
                            supports_tools: true,
                            supports_reasoning: true,
                        },
                        mimo_providers::provider::ModelInfo {
                            id: "mimo-v2.5-flash".to_string(),
                            provider: "mimo".to_string(),
                            supports_tools: true,
                            supports_reasoning: true,
                        },
                    ];
                    self.set_status("Using fallback model list", StatusType::Warning);
                }
            }
        }
        self.models_fetch_needed = false;
    }

    pub fn create_session(&mut self) -> Result<()> {
        if let Some(ref mut sm) = self.session_manager {
            let session = sm.create_session();
            self.current_session = Some(session);
            self.messages.clear();
            return Ok(());
        }
        Err(anyhow::anyhow!("SessionManager not initialized"))
    }

    pub fn set_status(&mut self, msg: &str, st: StatusType) {
        self.status_message = Some(StatusMessage {
            message: msg.to_string(),
            status_type: st,
            created_at: Instant::now(),
        });
    }

    pub fn check_status_timeout(&mut self) {
        if let Some(ref status) = self.status_message {
            if status.created_at.elapsed() > Duration::from_millis(STATUS_TIMEOUT_MS) {
                self.status_message = None;
            }
        }
    }

    pub async fn refresh_data(&mut self) {
        if self.last_data_refresh.elapsed() < Duration::from_millis(500) {
            return;
        }
        self.last_data_refresh = Instant::now();

        if let Some(ref mgr) = self.subagent_manager {
            self.cached_agents = mgr.list(None).await;
            self.cached_agent_count = mgr.get_running_count().await;
        }
        if let Some(ref sm) = self.session_manager {
            self.cached_sessions = sm.list_sessions().into_iter().cloned().collect();
        }
    }

    pub fn handle_key_normal(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Char('q') => {
                self.should_quit = true;
            }
            KeyCode::Char('i') => {
                self.mode = AppMode::Insert;
                self.input.set_placeholder(
                    "输入消息... (⌘Enter / Ctrl+Enter 发送, Esc 取消)".to_string(),
                );
            }
            KeyCode::Char(':') | KeyCode::Char('/') => {
                self.mode = AppMode::Command;
                self.command_buffer = "/".to_string();
                self.input
                    .set_placeholder("输入命令... (Enter 执行, Esc 取消)".to_string());
                self.input.clear();
            }
            KeyCode::Tab => {
                self.active_view = match self.active_view {
                    ActiveView::Chat => ActiveView::SubAgents,
                    ActiveView::SubAgents => ActiveView::History,
                    ActiveView::History => ActiveView::Chat,
                };
            }
            KeyCode::BackTab => {
                self.active_view = match self.active_view {
                    ActiveView::Chat => ActiveView::History,
                    ActiveView::SubAgents => ActiveView::Chat,
                    ActiveView::History => ActiveView::SubAgents,
                };
            }
            KeyCode::Char('j') | KeyCode::Down => match self.active_view {
                ActiveView::Chat => self.messages.scroll_down(self.scroll_amount),
                ActiveView::SubAgents => {
                    let count = self.get_agent_count();
                    self.agents_view.move_down(count);
                }
                ActiveView::History => {
                    let count = self.get_session_count();
                    self.history_view.move_down(count);
                }
            },
            KeyCode::Char('k') | KeyCode::Up => match self.active_view {
                ActiveView::Chat => self.messages.scroll_up(self.scroll_amount),
                ActiveView::SubAgents => self.agents_view.move_up(),
                ActiveView::History => self.history_view.move_up(),
            },
            KeyCode::Char('g') => {
                if self.active_view == ActiveView::Chat {
                    self.messages.scroll_to_top();
                }
            }
            KeyCode::Char('G') => {
                if self.active_view == ActiveView::Chat {
                    self.messages.scroll_to_bottom();
                }
            }
            KeyCode::Char('?') => {
                self.mode = AppMode::HelpPopup;
            }
            KeyCode::Char('c') => {
                if self.active_view == ActiveView::SubAgents {
                    if let Some(ref mgr) = self.subagent_manager {
                        let idx = self.agents_view.selected_index;
                        if let Some(agent) = self.cached_agents.get(idx) {
                            let agent_id = agent.id.clone();
                            let mgr_clone = mgr.clone();
                            let short_id = agent_id[..agent_id.len().min(8)].to_string();
                            let handle = tokio::runtime::Handle::current();
                            handle.spawn(async move {
                                let _ = mgr_clone.cancel(&agent_id).await;
                            });
                            self.set_status(
                                &format!("Cancelling agent {}", short_id),
                                StatusType::Warning,
                            );
                        }
                    }
                }
            }
            KeyCode::Char('d') => {
                if self.active_view == ActiveView::SubAgents {
                    self.agents_view.toggle_detail();
                }
            }
            KeyCode::Esc => {
                if self.is_streaming {
                    self.cancel_stream();
                }
            }
            _ => {}
        }

        if is_ctrl_or_cmd(key.modifiers) {
            match key.code {
                KeyCode::Char('c') => {
                    if self.is_streaming {
                        self.cancel_stream();
                    } else {
                        self.should_quit = true;
                    }
                }
                KeyCode::Char('l') => {
                    self.set_status("Screen cleared", StatusType::Info);
                }
                KeyCode::Char('d') => {
                    self.messages.scroll_up(self.scroll_amount * 3);
                }
                KeyCode::Char('u') => {
                    self.messages.scroll_down(self.scroll_amount * 3);
                }
                _ => {}
            }
        }
    }

    pub fn handle_key_insert(&mut self, key: KeyEvent) {
        match self.input.handle_key(key) {
            InputAction::Submit(text) => {
                if self.is_streaming {
                    return;
                }
                self.set_status("Sending message...", StatusType::Info);
                self.spawn_send_message(text);
                self.mode = AppMode::Normal;
            }
            InputAction::Cancel => {
                self.input.clear();
                self.command_buffer.clear();
                if self.is_streaming {
                    self.cancel_stream();
                }
                self.mode = AppMode::Normal;
            }
            InputAction::None => {}
        }
    }

    pub fn handle_key_command(&mut self, key: KeyEvent) {
        if self.mode == AppMode::Command {
            match key.code {
                KeyCode::Esc => {
                    self.command_buffer.clear();
                    self.completion_candidates.clear();
                    self.completion_index = 0;
                    self.mode = AppMode::Normal;
                    return;
                }
                KeyCode::Enter => {
                    let cmd_text = self.command_buffer.clone();
                    self.command_buffer.clear();
                    self.completion_candidates.clear();
                    self.completion_index = 0;
                    self.execute_command(&cmd_text);
                    self.mode = AppMode::Normal;
                    return;
                }
                KeyCode::Backspace => {
                    self.completion_candidates.clear();
                    self.completion_index = 0;
                    if self.command_buffer.len() > 1 {
                        self.command_buffer.pop();
                    } else {
                        self.command_buffer.clear();
                        self.mode = AppMode::Normal;
                    }
                    return;
                }
                KeyCode::Tab => {
                    if self.command_buffer.is_empty() {
                        return;
                    }
                    if self.completion_candidates.is_empty() {
                        let model_ids: Vec<String> = self
                            .model_list
                            .iter()
                            .map(|m| m.id.clone())
                            .collect();
                        let session_ids: Vec<String> = self
                            .cached_sessions
                            .iter()
                            .map(|s| s.id.clone())
                            .collect();
                        self.completion_candidates = get_command_completions(
                            &self.command_buffer,
                            &model_ids,
                            &session_ids,
                        );
                        self.completion_index = 0;
                    }
                    if !self.completion_candidates.is_empty() {
                        let idx = self.completion_index % self.completion_candidates.len();
                        self.command_buffer = self.completion_candidates[idx].clone();
                        self.completion_index = idx + 1;
                    }
                    return;
                }
                KeyCode::Char(c) => {
                    self.completion_candidates.clear();
                    self.completion_index = 0;
                    self.command_buffer.push(c);
                    return;
                }
                _ => return,
            }
        }
    }

    pub fn handle_key_popup(&mut self, key: KeyEvent) {
        match self.mode {
            AppMode::HelpPopup | AppMode::ToolsPopup => {
                self.mode = AppMode::Normal;
            }
            AppMode::MemoryPopup => match key.code {
                KeyCode::Char('q') | KeyCode::Esc => {
                    self.mode = AppMode::Normal;
                }
                KeyCode::Char('c') => {
                    self.execute_command("/memory clear");
                    self.mode = AppMode::Normal;
                }
                _ => {
                    self.mode = AppMode::Normal;
                }
            },
            AppMode::ApprovalPending => match key.code {
                KeyCode::Char('a') => {
                    self.approval_request = None;
                    self.set_status("Tool approved", StatusType::Success);
                    self.mode = AppMode::Normal;
                }
                KeyCode::Char('r') => {
                    self.approval_request = None;
                    self.set_status("Tool rejected", StatusType::Warning);
                    self.mode = AppMode::Normal;
                }
                KeyCode::Char('A') => {
                    self.approval_request = None;
                    self.set_status(
                        "All future tools approved for this session",
                        StatusType::Info,
                    );
                    self.mode = AppMode::Normal;
                }
                KeyCode::Char('q') | KeyCode::Esc => {
                    self.approval_request = None;
                    self.mode = AppMode::Normal;
                }
                _ => {}
            },
            _ => {}
        }
    }

    pub fn handle_key(&mut self, key: KeyEvent) {
        match self.mode {
            AppMode::Normal => self.handle_key_normal(key),
            AppMode::Insert => self.handle_key_insert(key),
            AppMode::Command => self.handle_key_command(key),
            AppMode::HelpPopup
            | AppMode::ToolsPopup
            | AppMode::MemoryPopup
            | AppMode::ApprovalPending => {
                self.handle_key_popup(key);
            }
        }
    }

    pub fn handle_mouse(&mut self, _kind: MouseEventKind, _column: u16, _row: u16) {}

    fn execute_command(&mut self, input: &str) {
        let cmd = parse_command(input);
        match cmd {
            Command::Help => {
                self.mode = AppMode::HelpPopup;
            }
            Command::Memory(subcmd) => match subcmd {
                MemorySubCommand::Show => {
                    self.mode = AppMode::MemoryPopup;
                }
                MemorySubCommand::Add(content) => {
                    if let Some(ref mut mm) = self.memory_manager {
                        let result = mm.add(MemoryTarget::Memory, &content);
                        if result.ok {
                            self.set_status(
                                &format!("Memory added ({}% used)", result.usage_pct),
                                StatusType::Success,
                            );
                        } else {
                            self.set_status(&result.message, StatusType::Error);
                        }
                    }
                }
                MemorySubCommand::Clear => {
                    self.set_status(
                        "Memory clear not available - manually delete ~/.mimo/MEMORY.md",
                        StatusType::Warning,
                    );
                }
            },
            Command::Tools => {
                self.mode = AppMode::ToolsPopup;
            }
            Command::Sessions => {
                self.active_view = ActiveView::History;
            }
            Command::SwitchSession(_id) => {
                self.set_status("Session switching not yet implemented", StatusType::Warning);
            }
            Command::Clear => {
                self.messages.clear();
                if let Some(ref mut sm) = self.session_manager {
                    let session = sm.create_session();
                    self.current_session = Some(session);
                }
                self.set_status("Conversation cleared", StatusType::Success);
            }
            Command::Compact => {
                self.set_status("Context compaction triggered", StatusType::Info);
            }
            Command::ModelSwitch(model_name) => {
                if self.model_list.iter().any(|m| m.id == model_name) {
                    self.model = model_name.clone();
                    self.set_status(
                        &format!("Model switched to: {}", model_name),
                        StatusType::Success,
                    );
                } else {
                    let available: Vec<String> =
                        self.model_list.iter().map(|m| m.id.clone()).collect();
                    self.set_status(
                        &format!(
                            "Unknown model: {}. Available: {}",
                            model_name,
                            available.join(", ")
                        ),
                        StatusType::Error,
                    );
                }
            }
            Command::Models => {
                if self.model_list.is_empty() {
                    self.set_status("No models loaded yet", StatusType::Warning);
                } else {
                    let models_str: Vec<String> = self
                        .model_list
                        .iter()
                        .map(|m| {
                            let current = if m.id == self.model { " ◀" } else { "" };
                            format!("{}{}", m.id, current)
                        })
                        .collect();
                    self.set_status(
                        &format!("Models: {}", models_str.join(" | ")),
                        StatusType::Info,
                    );
                }
            }
            Command::Quit => {
                self.should_quit = true;
            }
            Command::Unknown(cmd) => {
                self.set_status(&format!("Unknown command: {}", cmd), StatusType::Warning);
            }
        }
    }

    fn get_agent_count(&self) -> usize {
        self.cached_agents.len()
    }

    fn get_session_count(&self) -> usize {
        self.cached_sessions.len()
    }

    fn cancel_stream(&mut self) {
        if let Some(tx) = self.stream_cancel.take() {
            let _ = tx.send(true);
        }
        self.is_streaming = false;
        self.stream_rx = None;
        self.stream_msg_id = None;
        self.set_status("Stream cancelled", StatusType::Warning);
    }

    fn spawn_send_message(&mut self, text: String) {
        if text.trim().is_empty() {
            return;
        }

        if let Some(memory_content) = is_quick_memory(&text) {
            if let Some(ref mut mm) = self.memory_manager {
                let result = mm.add(MemoryTarget::Memory, &memory_content);
                if result.ok {
                    self.set_status(
                        &format!("Quick memory added ({}% used)", result.usage_pct),
                        StatusType::Success,
                    );
                } else {
                    self.set_status(&result.message, StatusType::Error);
                }
            }
            return;
        }

        let _user_msg_id = self.messages.add_message(MessageRole::User, text.clone());

        self.total_input_tokens += text.len().max(1) / 4;

        if let Some(ref mut session) = self.current_session {
            session.add_message(MessageRole::User, text.clone());
        }

        let stream_msg_id = self
            .messages
            .add_message(MessageRole::Assistant, String::new());

        self.is_streaming = true;
        self.stream_msg_id = Some(stream_msg_id.clone());

        let (stream_tx, stream_rx) = mpsc::unbounded_channel();
        self.stream_rx = Some(stream_rx);

        let (cancel_tx, cancel_rx) = tokio::sync::watch::channel(false);
        self.stream_cancel = Some(cancel_tx);

        let memory_block = self
            .memory_manager
            .as_ref()
            .and_then(|mm| mm.get_system_prompt_block());

        let provider = self.provider.clone();
        let model = self.model.clone();
        let tool_registry = self.tool_registry.clone();
        let tool_runner = self.tool_runner.clone();
        let subagent_manager = self.subagent_manager.clone();
        let session_messages = self.current_session.as_ref().map(|s| s.messages.clone());

        let handle = tokio::runtime::Handle::current();
        handle.spawn(async move {
            let tools = if let Some(ref registry) = tool_registry {
                build_tools_from_registry(registry)
            } else {
                Vec::new()
            };

            let mut messages: Vec<ProviderMessage> = Vec::new();

            let system_content = if let Some(memory_block) = memory_block {
                format!(
                    "你是一个智能编程助手 MiMo。你可以使用工具来完成用户的请求。\n\
                     你可以使用 agent_spawn 工具启动子智能体来处理复杂子任务，\n\
                     使用 agent_wait 等待子智能体完成，使用 agent_list 查看所有子智能体状态。\n\n{}",
                    memory_block
                )
            } else {
                "你是一个智能编程助手 MiMo。你可以使用工具来完成用户的请求。\n\
                 你可以使用 agent_spawn 工具启动子智能体来处理复杂子任务，\n\
                 使用 agent_wait 等待子智能体完成，使用 agent_list 查看所有子智能体状态。".to_string()
            };
            messages.push(ProviderMessage {
                role: ProviderMessageRole::System,
                content: system_content,
                name: None,
                tool_calls: None,
            });

            if let Some(session_msgs) = &session_messages {
                for m in session_msgs {
                    messages.push(ProviderMessage {
                        role: convert_role(&m.role),
                        content: m.content.clone(),
                        name: None,
                        tool_calls: None,
                    });
                }
            }

            let provider = match provider {
                Some(p) => p,
                None => {
                    let _ = stream_tx.send(StreamEvent::Error("Provider not initialized".to_string()));
                    return;
                }
            };

            let options = ChatOptions {
                temperature: Some(0.7),
                max_tokens: Some(4096),
                top_p: None,
                thinking: Some(ThinkingOptions {
                    enabled: true,
                    effort: Some("medium".to_string()),
                }),
                tools: if tools.is_empty() { None } else { Some(tools.clone()) },
            };

            const MAX_TURNS: usize = 10;
            let mut turn = 0;

            loop {
                if turn >= MAX_TURNS {
                    let _ = stream_tx.send(StreamEvent::Error("Max turns reached".to_string()));
                    return;
                }

                if *cancel_rx.borrow() {
                    return;
                }

                turn += 1;

                let mut collected_content = String::new();
                let mut collected_tool_calls: Vec<(String, String, String)> = Vec::new();

                match provider.chat_completions_stream(messages.clone(), &model, options.clone()).await {
                    Ok(mut chunk_rx) => {
                        while let Some(chunk_result) = chunk_rx.recv().await {
                            if *cancel_rx.borrow() {
                                return;
                            }

                            match chunk_result {
                                Ok(chunk) => {
                                    match chunk.delta {
                                        StreamDelta::Thinking(text) => {
                                            let _ = stream_tx.send(StreamEvent::ThinkingDelta(text));
                                        }
                                        StreamDelta::Content(text) => {
                                            collected_content.push_str(&text);
                                            let _ = stream_tx.send(StreamEvent::ContentDelta(text));
                                        }
                                        StreamDelta::ToolCallStart { id, name, arguments } => {
                                            collected_tool_calls.push((id.clone(), name.clone(), arguments.clone()));
                                            let _ = stream_tx.send(StreamEvent::ToolCallStart {
                                                id: id.clone(),
                                                name: name.clone(),
                                                arguments: arguments.clone(),
                                            });
                                        }
                                        StreamDelta::ToolCallDelta { .. } => {}
                                        StreamDelta::Done => {
                                            break;
                                        }
                                    }
                                }
                                Err(e) => {
                                    let _ = stream_tx.send(StreamEvent::Error(format!("Stream Error: {}", e)));
                                    return;
                                }
                            }
                        }
                    }
                    Err(e) => {
                        let _ = stream_tx.send(StreamEvent::Error(format!("API Error: {}", e)));
                        return;
                    }
                }

                if *cancel_rx.borrow() {
                    return;
                }

                if collected_tool_calls.is_empty() {
                    let _ = stream_tx.send(StreamEvent::Complete);
                    return;
                }

                let mut tool_result_entries: Vec<ProviderMessage> = Vec::new();
                let mut tool_calls_for_msg: Vec<mimo_providers::provider::ToolCall> = Vec::new();

                for (tc_id, tc_name, tc_args) in &collected_tool_calls {
                    tool_calls_for_msg.push(mimo_providers::provider::ToolCall {
                        id: tc_id.clone(),
                        name: tc_name.clone(),
                        arguments: HashMap::new(),
                    });

                    let result = if is_subagent_tool(tc_name) {
                        handle_subagent_tool(
                            tc_id, tc_name, tc_args,
                            &subagent_manager, &provider, &tool_registry, &tool_runner,
                            &model, &stream_tx, &cancel_rx,
                        ).await
                    } else if let Some(ref registry) = tool_registry {
                        if let Some(descriptor) = registry.get(tc_name) {
                            let args_value: serde_json::Value = serde_json::from_str(tc_args).unwrap_or_default();
                            let core_call = CoreToolCall {
                                id: tc_id.clone(),
                                name: tc_name.clone(),
                                arguments: args_value,
                            };
                            if let Some(ref runner) = tool_runner {
                                let result = runner.execute(core_call, descriptor).await;
                                Some(result)
                            } else {
                                Some(CoreToolResult::Error(ToolError::new("runner_error", "ToolRunner not available".to_string())))
                            }
                        } else {
                            Some(CoreToolResult::Error(ToolError::new("tool_error", format!("Unknown tool: {}", tc_name))))
                        }
                    } else {
                        Some(CoreToolResult::Error(ToolError::new("registry_error", "ToolRegistry not available".to_string())))
                    };

                    if let Some(res) = result {
                        match res {
                            CoreToolResult::Success(s) => {
                                let _ = stream_tx.send(StreamEvent::ToolCallResult {
                                    id: tc_id.clone(),
                                    result: s.clone(),
                                    success: true,
                                });
                                tool_result_entries.push(ProviderMessage {
                                    role: ProviderMessageRole::Tool,
                                    content: s,
                                    name: Some(tc_name.clone()),
                                    tool_calls: None,
                                });
                            }
                            CoreToolResult::Error(e) => {
                                let _ = stream_tx.send(StreamEvent::ToolCallResult {
                                    id: tc_id.clone(),
                                    result: e.message.clone(),
                                    success: false,
                                });
                                tool_result_entries.push(ProviderMessage {
                                    role: ProviderMessageRole::Tool,
                                    content: e.message,
                                    name: Some(tc_name.clone()),
                                    tool_calls: None,
                                });
                            }
                        }
                    }
                }

                messages.push(ProviderMessage {
                    role: ProviderMessageRole::Assistant,
                    content: collected_content,
                    name: None,
                    tool_calls: Some(tool_calls_for_msg),
                });

                for entry in tool_result_entries {
                    messages.push(entry);
                }
            }
        });
    }
}

fn is_subagent_tool(name: &str) -> bool {
    matches!(
        name,
        "agent_spawn" | "agent_wait" | "agent_result" | "agent_cancel" | "agent_list"
    )
}

async fn handle_subagent_tool(
    _tc_id: &str,
    tc_name: &str,
    tc_args: &str,
    subagent_manager: &Option<Arc<SubAgentManager>>,
    provider: &Arc<MimoProvider>,
    tool_registry: &Option<Arc<ToolRegistry>>,
    tool_runner: &Option<Arc<ToolRunner>>,
    model: &str,
    stream_tx: &mpsc::UnboundedSender<StreamEvent>,
    _cancel_rx: &tokio::sync::watch::Receiver<bool>,
) -> Option<CoreToolResult> {
    let mgr = subagent_manager.as_ref()?;
    let args: serde_json::Value = serde_json::from_str(tc_args).ok()?;

    match tc_name {
        "agent_spawn" => {
            let role_str: String = args
                .get("role")
                .and_then(|v| v.as_str())
                .unwrap_or("general")
                .to_string();
            let task: String = args
                .get("task")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            if task.is_empty() {
                return Some(CoreToolResult::Error(ToolError::new(
                    "agent_spawn_error",
                    "Task description is required".to_string(),
                )));
            }
            let role = SubAgentRole::from_str(&role_str).unwrap_or(SubAgentRole::General);

            let agent_id = mgr.spawn(role, task.clone()).await;
            let _ = mgr.start(&agent_id).await;

            let agent_mgr = mgr.clone();
            let agent_provider = provider.clone();
            let agent_tool_registry = tool_registry.clone();
            let agent_tool_runner = tool_runner.clone();
            let agent_model = model.to_string();
            let agent_tx = stream_tx.clone();
            let agent_id_clone = agent_id.clone();
            let spawn_task = task.clone();

            tokio::spawn(async move {
                run_sub_agent(
                    agent_id_clone,
                    role,
                    spawn_task,
                    agent_provider,
                    agent_tool_registry,
                    agent_tool_runner,
                    agent_mgr,
                    agent_model,
                    agent_tx,
                )
                .await;
            });

            Some(CoreToolResult::Success(format!(
                "Sub-agent spawned successfully. agent_id: {}, role: {:?}, task: {}. \
                 Use agent_wait to wait for its completion, agent_list to see all agents.",
                agent_id, role, task
            )))
        }
        "agent_wait" => {
            let agent_id: String = args
                .get("agent_id")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            if agent_id.is_empty() {
                return Some(CoreToolResult::Error(ToolError::new(
                    "agent_wait_error",
                    "agent_id is required".to_string(),
                )));
            }
            let timeout = args.get("timeout_secs").and_then(|v| v.as_u64());

            let _ = stream_tx.send(StreamEvent::AgentProgress {
                agent_id: agent_id.to_string(),
                status: "waiting".to_string(),
                progress: 0,
                message: Some("Waiting for sub-agent to complete...".to_string()),
            });

            match mgr.wait(&agent_id, timeout).await {
                Ok(Some(result)) => {
                    let output = serde_json::to_string_pretty(&result).unwrap_or_default();
                    let _ = stream_tx.send(StreamEvent::AgentProgress {
                        agent_id: agent_id.clone(),
                        status: "completed".to_string(),
                        progress: 100,
                        message: Some("Sub-agent completed".to_string()),
                    });
                    Some(CoreToolResult::Success(format!(
                        "Sub-agent {} completed.\nResult:\n{}",
                        agent_id, output
                    )))
                }
                Ok(None) => Some(CoreToolResult::Success(format!(
                    "Sub-agent {} timed out or is not yet complete. Use agent_result to check later.",
                    agent_id
                ))),
                Err(e) => Some(CoreToolResult::Error(ToolError::new(
                    "agent_wait_error",
                    e.to_string(),
                ))),
            }
        }
        "agent_result" => {
            let agent_id: String = args
                .get("agent_id")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            if agent_id.is_empty() {
                return Some(CoreToolResult::Error(ToolError::new(
                    "agent_result_error",
                    "agent_id is required".to_string(),
                )));
            }
            match mgr.result(&agent_id).await {
                Ok(Some(result)) => {
                    let output = serde_json::to_string_pretty(&result).unwrap_or_default();
                    Some(CoreToolResult::Success(format!(
                        "Sub-agent {} result:\n{}",
                        agent_id, output
                    )))
                }
                Ok(None) => {
                    let agent = mgr.get_agent(&agent_id).await;
                    match agent {
                        Some(a) => Some(CoreToolResult::Success(format!(
                            "Sub-agent {} is {:?}, progress: {}%. {}",
                            agent_id,
                            a.status,
                            a.progress,
                            a.progress_message.as_deref().unwrap_or("")
                        ))),
                        None => Some(CoreToolResult::Error(ToolError::new(
                            "agent_result_error",
                            format!("SubAgent {} not found", agent_id),
                        ))),
                    }
                }
                Err(e) => Some(CoreToolResult::Error(ToolError::new(
                    "agent_result_error",
                    e.to_string(),
                ))),
            }
        }
        "agent_cancel" => {
            let agent_id: String = args
                .get("agent_id")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            if agent_id.is_empty() {
                return Some(CoreToolResult::Error(ToolError::new(
                    "agent_cancel_error",
                    "agent_id is required".to_string(),
                )));
            }
            match mgr.cancel(&agent_id).await {
                Ok(()) => {
                    let _ = stream_tx.send(StreamEvent::AgentProgress {
                        agent_id: agent_id.to_string(),
                        status: "cancelled".to_string(),
                        progress: 0,
                        message: Some("Sub-agent cancelled".to_string()),
                    });
                    Some(CoreToolResult::Success(format!(
                        "Sub-agent {} cancelled",
                        agent_id
                    )))
                }
                Err(e) => Some(CoreToolResult::Error(ToolError::new(
                    "agent_cancel_error",
                    e.to_string(),
                ))),
            }
        }
        "agent_list" => {
            let agents = mgr.list(None).await;
            if agents.is_empty() {
                Some(CoreToolResult::Success("No sub-agents found.".to_string()))
            } else {
                let listing: Vec<String> = agents
                    .iter()
                    .map(|a| {
                        format!(
                            "  {} | {:?} | {:?} | {}% | {}",
                            &a.id[..a.id.len().min(8)],
                            a.role,
                            a.status,
                            a.progress,
                            a.task.chars().take(60).collect::<String>()
                        )
                    })
                    .collect();
                Some(CoreToolResult::Success(format!(
                    "Sub-agents ({}):\n{}",
                    agents.len(),
                    listing.join("\n")
                )))
            }
        }
        _ => Some(CoreToolResult::Error(ToolError::new(
            "agent_error",
            format!("Unknown sub-agent tool: {}", tc_name),
        ))),
    }
}

async fn run_sub_agent(
    agent_id: String,
    role: SubAgentRole,
    task: String,
    provider: Arc<MimoProvider>,
    tool_registry: Option<Arc<ToolRegistry>>,
    tool_runner: Option<Arc<ToolRunner>>,
    subagent_manager: Arc<SubAgentManager>,
    model: String,
    stream_tx: mpsc::UnboundedSender<StreamEvent>,
) {
    let tools = if let Some(ref registry) = tool_registry {
        build_tools_from_registry(registry)
    } else {
        Vec::new()
    };

    let sub_model = if model.contains("pro") {
        "mimo-v2.5-flash".to_string()
    } else {
        model.clone()
    };

    let system_prompt = format!(
        "{}\n\nYour task: {}\n\
         Complete this task thoroughly. Use tools as needed.\n\
         When finished, provide a clear summary of what you did.",
        role.system_prompt(),
        task
    );

    let mut messages: Vec<ProviderMessage> = Vec::new();
    messages.push(ProviderMessage {
        role: ProviderMessageRole::System,
        content: system_prompt,
        name: None,
        tool_calls: None,
    });
    messages.push(ProviderMessage {
        role: ProviderMessageRole::User,
        content: task.clone(),
        name: None,
        tool_calls: None,
    });

    let _ = stream_tx.send(StreamEvent::AgentProgress {
        agent_id: agent_id.clone(),
        status: "running".to_string(),
        progress: 10,
        message: Some(format!("Sub-agent {:?} starting: {}", role, task)),
    });

    let _ = subagent_manager
        .update_progress(&agent_id, 10, Some("Starting...".to_string()))
        .await;

    let options = ChatOptions {
        temperature: Some(0.5),
        max_tokens: Some(4096),
        top_p: None,
        thinking: Some(ThinkingOptions {
            enabled: true,
            effort: Some("low".to_string()),
        }),
        tools: if tools.is_empty() {
            None
        } else {
            Some(tools.clone())
        },
    };

    const MAX_TURNS: usize = 8;
    let mut final_summary = String::new();
    let mut all_content = String::new();

    for turn in 0..MAX_TURNS {
        let progress = 10 + ((turn as f32 / MAX_TURNS as f32) * 80.0) as u8;
        let _ = subagent_manager
            .update_progress(
                &agent_id,
                progress,
                Some(format!("Turn {}/{}", turn + 1, MAX_TURNS)),
            )
            .await;
        let _ = stream_tx.send(StreamEvent::AgentProgress {
            agent_id: agent_id.clone(),
            status: "running".to_string(),
            progress,
            message: Some(format!("Turn {}/{}", turn + 1, MAX_TURNS)),
        });

        let mut turn_content = String::new();
        let mut turn_tool_calls: Vec<(String, String, String)> = Vec::new();

        match provider
            .chat_completions_stream(messages.clone(), &sub_model, options.clone())
            .await
        {
            Ok(mut chunk_rx) => {
                while let Some(chunk_result) = chunk_rx.recv().await {
                    match chunk_result {
                        Ok(chunk) => match chunk.delta {
                            StreamDelta::Content(text) => {
                                turn_content.push_str(&text);
                                let _ = subagent_manager.append_content(&agent_id, &text).await;
                            }
                            StreamDelta::ToolCallStart {
                                id,
                                name,
                                arguments,
                            } => {
                                turn_tool_calls.push((id.clone(), name.clone(), arguments.clone()));
                            }
                            _ => {}
                        },
                        Err(_) => break,
                    }
                }
            }
            Err(e) => {
                let _ = subagent_manager
                    .fail_subagent(&agent_id, format!("API error: {}", e))
                    .await;
                let _ = stream_tx.send(StreamEvent::AgentProgress {
                    agent_id: agent_id.clone(),
                    status: "failed".to_string(),
                    progress: 0,
                    message: Some(format!("API error: {}", e)),
                });
                return;
            }
        }

        all_content.push_str(&turn_content);

        if turn_tool_calls.is_empty() {
            final_summary = turn_content;
            break;
        }

        let mut tool_calls_for_msg: Vec<mimo_providers::provider::ToolCall> = Vec::new();
        let mut tool_results: Vec<ProviderMessage> = Vec::new();

        for (tc_id, tc_name, tc_args) in &turn_tool_calls {
            tool_calls_for_msg.push(mimo_providers::provider::ToolCall {
                id: tc_id.clone(),
                name: tc_name.clone(),
                arguments: HashMap::new(),
            });

            if let Some(ref registry) = tool_registry {
                if let Some(descriptor) = registry.get(tc_name) {
                    if !is_subagent_tool(tc_name) {
                        let args_value: serde_json::Value =
                            serde_json::from_str(tc_args).unwrap_or_default();
                        let core_call = CoreToolCall {
                            id: tc_id.clone(),
                            name: tc_name.clone(),
                            arguments: args_value,
                        };
                        if let Some(ref runner) = tool_runner {
                            let result = runner.execute(core_call, descriptor).await;
                            match result {
                                CoreToolResult::Success(s) => {
                                    tool_results.push(ProviderMessage {
                                        role: ProviderMessageRole::Tool,
                                        content: s,
                                        name: Some(tc_name.clone()),
                                        tool_calls: None,
                                    });
                                }
                                CoreToolResult::Error(e) => {
                                    tool_results.push(ProviderMessage {
                                        role: ProviderMessageRole::Tool,
                                        content: e.message,
                                        name: Some(tc_name.clone()),
                                        tool_calls: None,
                                    });
                                }
                            }
                        }
                    }
                }
            }
        }

        messages.push(ProviderMessage {
            role: ProviderMessageRole::Assistant,
            content: turn_content,
            name: None,
            tool_calls: Some(tool_calls_for_msg),
        });

        for tr in tool_results {
            messages.push(tr);
        }
    }

    if final_summary.is_empty() {
        final_summary = all_content;
    }

    let result = SubAgentResult {
        summary: final_summary.chars().take(2000).collect(),
        changes: Vec::new(),
        evidence: Vec::new(),
        risks: Vec::new(),
        blockers: Vec::new(),
    };

    let _ = subagent_manager.complete_subagent(&agent_id, result).await;
    let _ = stream_tx.send(StreamEvent::AgentProgress {
        agent_id: agent_id.clone(),
        status: "completed".to_string(),
        progress: 100,
        message: Some("Sub-agent completed".to_string()),
    });
}

impl App {
    pub fn process_stream_events(&mut self) {
        let mut events = Vec::new();
        if let Some(ref mut rx) = self.stream_rx {
            while let Ok(event) = rx.try_recv() {
                events.push(event);
            }
        }

        for event in events {
            match event {
                StreamEvent::ThinkingDelta(text) => {
                    self.thinking_active = true;
                    if let Some(ref msg_id) = self.stream_msg_id {
                        self.messages.add_thinking(msg_id, &text);
                    }
                }
                StreamEvent::ContentDelta(text) => {
                    if self.thinking_active {
                        self.thinking_active = false;
                        if let Some(ref msg_id) = self.stream_msg_id {
                            self.messages.finish_thinking(msg_id);
                        }
                    }
                    if let Some(ref msg_id) = self.stream_msg_id {
                        self.messages.append_content(msg_id, &text);
                    }
                    self.total_output_tokens += text.len().max(1) / 4;
                }
                StreamEvent::ToolCallStart {
                    id,
                    name,
                    arguments,
                } => {
                    if self.thinking_active {
                        self.thinking_active = false;
                        if let Some(ref msg_id) = self.stream_msg_id {
                            self.messages.finish_thinking(msg_id);
                        }
                    }
                    self.total_tool_calls += 1;
                    if let Some(ref msg_id) = self.stream_msg_id {
                        self.messages.add_tool_call(msg_id, &id, &name, &arguments);
                    }
                }
                StreamEvent::ToolCallResult {
                    id,
                    result,
                    success,
                } => {
                    if let Some(ref msg_id) = self.stream_msg_id {
                        self.messages
                            .update_tool_result(msg_id, &id, result, success);
                    }
                }
                StreamEvent::AgentProgress {
                    agent_id,
                    status: _,
                    progress,
                    message,
                } => {
                    if let Some(ref msg_id) = self.stream_msg_id {
                        let msg_text = message.unwrap_or_default();
                        let info = format!(
                            "🤖 Sub-agent {} ({}%): {}",
                            &agent_id[..agent_id.len().min(8)],
                            progress,
                            msg_text
                        );
                        self.messages
                            .append_content(msg_id, &format!("\n[{}]\n", info));
                    }
                }
                StreamEvent::Complete => {
                    if self.thinking_active {
                        self.thinking_active = false;
                        if let Some(ref msg_id) = self.stream_msg_id {
                            self.messages.finish_thinking(msg_id);
                        }
                    }
                    self.is_streaming = false;
                    self.stream_msg_id = None;
                    self.turn_count += 1;
                    self.set_status("Response complete", StatusType::Success);
                }
                StreamEvent::Error(err) => {
                    if self.thinking_active {
                        self.thinking_active = false;
                        if let Some(ref msg_id) = self.stream_msg_id {
                            self.messages.finish_thinking(msg_id);
                        }
                    }
                    self.is_streaming = false;
                    self.stream_msg_id = None;
                    self.set_status(&err, StatusType::Error);
                }
            }
        }
    }

    pub fn render(&self, f: &mut ratatui::Frame) -> Option<ratatui::layout::Position> {
        use ratatui::layout::{Constraint, Direction, Layout, Position};

        let area = f.area();
        let main_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1),
                Constraint::Min(1),
                Constraint::Length(2),
            ])
            .split(area);

        let header = HeaderComponent::new();
        let memory_count = if let Some(ref mm) = self.memory_manager {
            mm.list_entries(MemoryTarget::Memory).len()
        } else {
            0
        };
        let memory_usage = if let Some(ref mm) = self.memory_manager {
            mm.memory_usage(MemoryTarget::Memory).pct
        } else {
            0
        };
        header.render(
            f,
            main_chunks[0],
            &self.theme,
            &self.model,
            self.memory_enabled,
            memory_count,
            memory_usage,
        );

        let content_area = main_chunks[1];
        let content_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(70), Constraint::Percentage(30)])
            .split(content_area);

        let left_area = content_chunks[0];
        let right_area = content_chunks[1];

        let mut cursor_pos: Option<Position> = None;
        match self.active_view {
            ActiveView::Chat => {
                cursor_pos = self.chat_view.render(
                    f,
                    left_area,
                    &self.theme,
                    &self.messages,
                    &self.input,
                    self.mode == AppMode::Insert || self.mode == AppMode::Command,
                    self.is_streaming,
                    self.approval_request.as_ref(),
                );
            }
            ActiveView::SubAgents => {
                self.agents_view
                    .render(f, left_area, &self.theme, &self.cached_agents);
            }
            ActiveView::History => {
                self.history_view
                    .render(f, left_area, &self.theme, &self.cached_sessions);
            }
        }

        let sidebar_height = right_area.height / 4;
        let sidebar_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(sidebar_height.max(3)),
                Constraint::Length(sidebar_height.max(3)),
                Constraint::Length(sidebar_height.max(3)),
                Constraint::Min(3),
            ])
            .split(right_area);

        let plan_content = {
            let mut items = Vec::new();
            if self.is_streaming {
                items.push("⏳ Waiting for response...".to_string());
            } else if self.turn_count > 0 {
                items.push(format!(
                    "Turn {} · {} tool calls",
                    self.turn_count, self.total_tool_calls
                ));
            }
            if self.memory_enabled {
                items.push(format!("Memory: {} entries", memory_count));
            }
            let msg_count = self.messages.messages.len();
            if msg_count > 0 {
                let user_count = self
                    .messages
                    .messages
                    .iter()
                    .filter(|m| matches!(m.role, MessageRole::User))
                    .count();
                let asst_count = self
                    .messages
                    .messages
                    .iter()
                    .filter(|m| matches!(m.role, MessageRole::Assistant))
                    .count();
                items.push(format!("{} user / {} assistant", user_count, asst_count));
            }
            if items.is_empty() {
                items.push("No active plan".to_string());
            }
            items
        };
        let plan_panel = SidebarPanel::new("Plan").with_content(plan_content);
        plan_panel.render(f, sidebar_chunks[0], &self.theme);

        let todos_content = {
            let mut items = Vec::new();
            let tool_count = self.total_tool_calls;
            if tool_count > 0 {
                items.push(format!("{} tool calls this session", tool_count));
            }
            if let Some(ref registry) = self.tool_registry {
                let tools = registry.list();
                items.push(format!("{} tools available", tools.len()));
            }
            if self.is_streaming {
                items.push("⏳ Response in progress".to_string());
            }
            if items.is_empty() {
                items.push("No todos".to_string());
            }
            items
        };
        let todos_panel = SidebarPanel::new("Todos").with_content(todos_content);
        todos_panel.render(f, sidebar_chunks[1], &self.theme);

        let tasks_content = {
            let mut items = Vec::new();
            items.push(format!(
                "turn {} · {} msgs",
                self.turn_count,
                self.messages.messages.len()
            ));
            let elapsed = self.session_start.elapsed();
            items.push(format!("session {}", format_duration(elapsed)));
            if self.total_tool_calls > 0 {
                items.push(format!("{} tools executed", self.total_tool_calls));
            }
            let running_tools: Vec<String> = self
                .messages
                .messages
                .iter()
                .filter_map(|m| m.tool_calls.as_ref())
                .flat_map(|tcs| tcs.iter())
                .filter(|tc| matches!(tc.status, ToolCallStatus::Running | ToolCallStatus::Pending))
                .map(|tc| format!("⚙ {}", tc.name))
                .collect();
            for rt in running_tools {
                items.push(rt);
            }
            items
        };
        let tasks_panel = SidebarPanel::new("Tasks").with_content(tasks_content);
        tasks_panel.render(f, sidebar_chunks[2], &self.theme);

        let agents_content = if self.cached_agents.is_empty() {
            vec!["No agents".to_string()]
        } else {
            self.cached_agents
                .iter()
                .map(|a| {
                    let status_icon = match a.status {
                        mimo_core::subagent::SubAgentStatus::Running => "▶",
                        mimo_core::subagent::SubAgentStatus::Completed => "✓",
                        mimo_core::subagent::SubAgentStatus::Failed => "✗",
                        mimo_core::subagent::SubAgentStatus::Pending => "⏳",
                        _ => "○",
                    };
                    let role_str = format!("{:?}", a.role);
                    format!(
                        "{} {} ({})",
                        status_icon,
                        role_str,
                        a.id.chars().take(8).collect::<String>()
                    )
                })
                .collect()
        };
        let agents_panel = SidebarPanel::new("Agents").with_content(agents_content);
        agents_panel.render(f, sidebar_chunks[3], &self.theme);

        let footer = FooterComponent::new();
        let mode_str = match self.mode {
            AppMode::Normal => "Normal",
            AppMode::Insert => "Insert",
            AppMode::Command => "Command",
            AppMode::HelpPopup => "Help",
            AppMode::ToolsPopup => "Tools",
            AppMode::MemoryPopup => "Memory",
            AppMode::ApprovalPending => "Approval",
        };
        let _active_view_str = match self.active_view {
            ActiveView::Chat => "Chat",
            ActiveView::SubAgents => "SubAgents",
            ActiveView::History => "History",
        };
        footer.render(
            f,
            main_chunks[2],
            &self.theme,
            &self.model,
            mode_str,
            self.turn_count,
            self.total_input_tokens,
            self.total_output_tokens,
            self.total_tool_calls,
            self.is_streaming,
        );

        if let Some(ref status) = self.status_message {
            use ratatui::style::Style;
            use ratatui::text::Span;
            use ratatui::widgets::Paragraph;

            let (color, icon) = match status.status_type {
                StatusType::Info => (self.theme.accent, "ℹ"),
                StatusType::Success => (self.theme.success, "✓"),
                StatusType::Warning => (self.theme.warning, "⚠"),
                StatusType::Error => (self.theme.error, "✗"),
            };

            let status_line = ratatui::text::Line::from(vec![Span::styled(
                format!(" {} {} ", icon, status.message),
                Style::default().fg(color),
            )]);
            let status_rect = ratatui::layout::Rect {
                x: area.x,
                y: area.y.saturating_add(area.height.saturating_sub(3)),
                width: area.width,
                height: 1,
            };
            f.render_widget(Paragraph::new(status_line), status_rect);
        }

        match self.mode {
            AppMode::HelpPopup => {
                self.help_view.render(f, area, &self.theme);
            }
            AppMode::ToolsPopup => {
                if let Some(ref registry) = self.tool_registry {
                    let names: Vec<String> =
                        registry.list().iter().map(|t| t.name.clone()).collect();
                    render_tools_popup(f, area, &self.theme, &names);
                }
            }
            AppMode::MemoryPopup => {
                if let Some(ref mm) = self.memory_manager {
                    let entries = mm.list_entries(MemoryTarget::Memory);
                    let usage = mm.memory_usage(MemoryTarget::Memory);
                    render_memory_popup(
                        f,
                        area,
                        &self.theme,
                        &entries,
                        usage.chars,
                        usage.limit_chars,
                    );
                }
            }
            _ => {}
        }

        if self.mode == AppMode::Command {
            use ratatui::style::Style;
            use ratatui::text::{Line, Span};
            use ratatui::widgets::Paragraph;

            let cmd_area = ratatui::layout::Rect {
                x: area.x + 1,
                y: area.y.saturating_add(area.height.saturating_sub(4)),
                width: area.width.saturating_sub(2).min(60),
                height: 1,
            };
            let cmd_line = Line::from(vec![
                Span::styled(": ", Style::default().fg(self.theme.accent)),
                Span::raw(&self.command_buffer),
            ]);
            f.render_widget(
                Paragraph::new(cmd_line).style(Style::default().bg(self.theme.muted)),
                cmd_area,
            );
        }

        cursor_pos
    }
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}

pub async fn run() -> Result<()> {
    terminal::enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(
        stdout,
        terminal::EnterAlternateScreen,
        event::EnableMouseCapture,
        event::EnableBracketedPaste,
    )?;

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let original_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let _ = terminal::disable_raw_mode();
        let _ = execute!(
            io::stdout(),
            terminal::LeaveAlternateScreen,
            event::DisableBracketedPaste
        );
        original_hook(info);
    }));

    let mut app = App::new();

    match app.initialize() {
        Ok(()) => {}
        Err(e) => {
            let _ = execute!(
                io::stdout(),
                terminal::LeaveAlternateScreen,
                event::DisableBracketedPaste
            );
            terminal::disable_raw_mode()?;
            eprintln!("Initialization error: {}", e);
            return Err(e);
        }
    }

    app.create_session()?;
    app.fetch_models_async().await;

    let res = run_app(&mut terminal, &mut app).await;

    execute!(
        io::stdout(),
        terminal::LeaveAlternateScreen,
        event::DisableBracketedPaste
    )?;
    terminal::disable_raw_mode()?;

    res
}

async fn run_app<B: ratatui::backend::Backend>(
    terminal: &mut Terminal<B>,
    app: &mut App,
) -> Result<()> {
    let poll_interval = Duration::from_millis(10);

    loop {
        app.check_status_timeout();
        app.process_stream_events();
        app.refresh_data().await;

        if app.models_fetch_needed {
            app.fetch_models_async().await;
        }

        let has_event = event::poll(poll_interval)?;
        if has_event {
            match event::read()? {
                Event::Key(key) => {
                    let ctrl_c = (key.code == KeyCode::Char('c') || key.code == KeyCode::Char('C'))
                        && is_ctrl_or_cmd(key.modifiers);
                    let esc = key.code == KeyCode::Esc;

                    if ctrl_c {
                        if app.is_streaming {
                            app.cancel_stream();
                        } else {
                            app.should_quit = true;
                        }
                        continue;
                    }
                    if esc
                        && (app.mode == AppMode::HelpPopup
                            || app.mode == AppMode::ToolsPopup
                            || app.mode == AppMode::MemoryPopup)
                    {
                        app.mode = AppMode::Normal;
                        continue;
                    }

                    app.handle_key(key);
                }
                Event::Mouse(mouse) => {
                    app.handle_mouse(mouse.kind, mouse.column, mouse.row);
                }
                Event::Paste(data) => {
                    if app.mode == AppMode::Insert || app.mode == AppMode::Command {
                        app.input.paste_text(&data);
                    }
                }
                Event::Resize(_, _) => {}
                _ => {}
            }
        }

        let mut cursor_pos: Option<ratatui::layout::Position> = None;
        terminal.draw(|f| {
            cursor_pos = app.render(f);
        })?;

        if let Some(pos) = cursor_pos {
            let _ = execute!(
                io::stdout(),
                crossterm::cursor::MoveTo(pos.x, pos.y),
                crossterm::cursor::Show,
            );
        } else {
            let _ = execute!(io::stdout(), crossterm::cursor::Hide);
        }

        if app.should_quit {
            break;
        }
    }

    Ok(())
}

fn convert_role(role: &MessageRole) -> ProviderMessageRole {
    match role {
        MessageRole::User => ProviderMessageRole::User,
        MessageRole::Assistant => ProviderMessageRole::Assistant,
        MessageRole::System => ProviderMessageRole::System,
        MessageRole::Tool => ProviderMessageRole::Tool,
    }
}

fn tool_to_definition(name: &str, desc: &str) -> ToolDefinition {
    let params = match name {
        "file_read" => serde_json::json!({
            "type": "object",
            "properties": {
                "path": {"type": "string", "description": "File path to read"}
            },
            "required": ["path"]
        }),
        "file_write" => serde_json::json!({
            "type": "object",
            "properties": {
                "path": {"type": "string", "description": "File path to write"},
                "content": {"type": "string", "description": "Content to write"}
            },
            "required": ["path", "content"]
        }),
        "shell" => serde_json::json!({
            "type": "object",
            "properties": {
                "command": {"type": "string", "description": "Shell command to execute"}
            },
            "required": ["command"]
        }),
        "grep" => serde_json::json!({
            "type": "object",
            "properties": {
                "pattern": {"type": "string", "description": "Regex pattern to search"},
                "path": {"type": "string", "description": "Directory or file to search in"}
            },
            "required": ["pattern", "path"]
        }),
        "glob" => serde_json::json!({
            "type": "object",
            "properties": {
                "pattern": {"type": "string", "description": "Glob pattern (e.g., **/*.rs)"},
                "path": {"type": "string", "description": "Base directory to search from"}
            },
            "required": ["pattern"]
        }),
        "git_status" => serde_json::json!({
            "type": "object",
            "properties": {
                "path": {"type": "string", "description": "Repository path (defaults to current directory)"}
            }
        }),
        "web_fetch" => serde_json::json!({
            "type": "object",
            "properties": {
                "url": {"type": "string", "description": "URL to fetch"}
            },
            "required": ["url"]
        }),
        "web_search" => serde_json::json!({
            "type": "object",
            "properties": {
                "query": {"type": "string", "description": "Search query"},
                "num": {"type": "integer", "description": "Number of results (default 5)"}
            },
            "required": ["query"]
        }),
        "remember" => serde_json::json!({
            "type": "object",
            "properties": {
                "note": {"type": "string", "description": "The note to save."},
                "target": {"type": "string", "enum": ["memory", "user"], "description": "Which memory store."}
            },
            "required": ["note"]
        }),
        "memory_replace" => serde_json::json!({
            "type": "object",
            "properties": {
                "old_text": {"type": "string", "description": "A unique substring to replace."},
                "content": {"type": "string", "description": "The new content."}
            },
            "required": ["old_text", "content"]
        }),
        "memory_remove" => serde_json::json!({
            "type": "object",
            "properties": {
                "old_text": {"type": "string", "description": "A unique substring to remove."}
            },
            "required": ["old_text"]
        }),
        "recall_archive" => serde_json::json!({
            "type": "object",
            "properties": {
                "query": {"type": "string", "description": "Search query for prior conversations"},
                "max_results": {"type": "integer", "description": "Maximum number of results (default 3, max 10)"}
            },
            "required": ["query"]
        }),
        "agent_spawn" => serde_json::json!({
            "type": "object",
            "properties": {
                "role": {"type": "string", "enum": ["general", "explore", "plan", "review", "implementer", "verifier", "custom"], "description": "Sub-agent role"},
                "task": {"type": "string", "description": "Task description for the sub-agent"}
            },
            "required": ["role", "task"]
        }),
        "agent_wait" => serde_json::json!({
            "type": "object",
            "properties": {
                "agent_id": {"type": "string", "description": "ID of the sub-agent to wait for"},
                "timeout_secs": {"type": "integer", "description": "Optional timeout in seconds"}
            },
            "required": ["agent_id"]
        }),
        "agent_result" => serde_json::json!({
            "type": "object",
            "properties": {
                "agent_id": {"type": "string", "description": "ID of the sub-agent to check"}
            },
            "required": ["agent_id"]
        }),
        "agent_cancel" => serde_json::json!({
            "type": "object",
            "properties": {
                "agent_id": {"type": "string", "description": "ID of the sub-agent to cancel"}
            },
            "required": ["agent_id"]
        }),
        "agent_list" => serde_json::json!({
            "type": "object",
            "properties": {},
            "required": []
        }),
        _ => serde_json::json!({"type": "object", "properties": {}}),
    };

    ToolDefinition {
        name: name.to_string(),
        description: desc.to_string(),
        parameters: params,
    }
}

fn build_tools_from_registry(registry: &ToolRegistry) -> Vec<ToolDefinition> {
    registry
        .list()
        .iter()
        .map(|t| tool_to_definition(&t.name, &t.description))
        .collect()
}

fn format_duration(duration: std::time::Duration) -> String {
    let secs = duration.as_secs();
    let ms = duration.subsec_millis();
    if secs >= 60 {
        let mins = secs / 60;
        let remain_secs = secs % 60;
        format!("{}m{}s", mins, remain_secs)
    } else if secs > 0 {
        if ms > 0 {
            format!("{}.{:01}s", secs, ms / 100)
        } else {
            format!("{}s", secs)
        }
    } else {
        format!("{}ms", ms)
    }
}
