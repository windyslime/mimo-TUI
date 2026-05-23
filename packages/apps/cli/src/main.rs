use anyhow::Result;
use mimo_core::{
    MemoryManager, RecallArchive, SessionManager, SubAgentManager, ToolRegistry,
    session::{MessageRole, Session},
    tools::{ToolCall as CoreToolCall, ToolResult as CoreToolResult, ToolRunner},
};
use mimo_providers::{
    LLMProvider, MimoProvider,
    provider::{
        ChatOptions, Message as ProviderMessage, MessageRole as ProviderMessageRole, ToolDefinition,
    },
};
use std::cell::RefCell;
use std::io::{self, Write};
use std::sync::Arc;

const DEFAULT_MODEL: &str = "mimo-v2.5-pro";
const MAX_TURN_DEPTH: usize = 10;

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
                "path": {"type": "string", "description": "Directory or file to search in"},
                "-n": {"type": "boolean", "description": "Show line numbers"},
                "-i": {"type": "boolean", "description": "Case insensitive"}
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
                "note": {"type": "string", "description": "The note to save. One sentence or short paragraph."},
                "target": {"type": "string", "enum": ["memory", "user"], "description": "Which memory store: 'memory' for agent notes (environment, project facts, learnings) or 'user' for user profile (preferences, habits, communication style). Default: 'memory'"}
            },
            "required": ["note"]
        }),
        "memory_replace" => serde_json::json!({
            "type": "object",
            "properties": {
                "target": {"type": "string", "enum": ["memory", "user"], "description": "Which memory store. Default: 'memory'"},
                "old_text": {"type": "string", "description": "A unique substring of the entry to replace. Must match exactly one entry."},
                "content": {"type": "string", "description": "The new content for the entry"}
            },
            "required": ["old_text", "content"]
        }),
        "memory_remove" => serde_json::json!({
            "type": "object",
            "properties": {
                "target": {"type": "string", "enum": ["memory", "user"], "description": "Which memory store. Default: 'memory'"},
                "old_text": {"type": "string", "description": "A unique substring of the entry to remove. Must match exactly one entry."}
            },
            "required": ["old_text"]
        }),
        "recall_archive" => serde_json::json!({
            "type": "object",
            "properties": {
                "query": {"type": "string", "description": "Search query for prior conversations"},
                "max_results": {"type": "integer", "description": "Maximum number of results (default 3, max 10)"},
                "session_id": {"type": "string", "description": "Session ID to search (optional)"},
                "session_content": {"type": "string", "description": "Session content to search (optional)"}
            },
            "required": ["query"]
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

    eprintln!("请设置 MIMO_API_KEY 环境变量：");
    eprintln!("  export MIMO_API_KEY=你的API密钥");
    eprintln!("或者将密钥保存到 ~/.mimo/api_key 文件中");
    eprintln!("或者在 ~/.mimo/config.toml 中设置 api_key");
    anyhow::bail!("MIMO_API_KEY not set")
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let args: Vec<String> = std::env::args().collect();

    let help = args.contains(&"-h".to_string()) || args.contains(&"--help".to_string());
    if help {
        print_usage();
        return Ok(());
    }

    let version = args.contains(&"-v".to_string()) || args.contains(&"--version".to_string());
    if version {
        println!("MiMo-TUI v{}", env!("CARGO_PKG_VERSION"));
        return Ok(());
    }

    let start_tui = args.contains(&"-s".to_string())
        || args.contains(&"--start-tui".to_string())
        || args.get(1).map(|s| s.as_str()) == Some("tui");
    if start_tui {
        mimo_tui::run().await?;
        return Ok(());
    }

    let debug = args.contains(&"-d".to_string())
        || args.contains(&"--debug".to_string())
        || args.contains(&"--repl".to_string())
        || args.get(1).map(|s| s.as_str()) == Some("repl");
    if debug {
        debug_repl().await?;
        return Ok(());
    }

    if args.get(1).map(|s| s.as_str()) == Some("chat") {
        if let Some(msg) = args.get(2) {
            oneshot_chat(msg).await?;
            return Ok(());
        }
        eprintln!("Usage: mimo chat \"<message>\"");
        return Ok(());
    }

    if let Some(pos) = args.iter().position(|a| a == "-c" || a == "--chat") {
        if let Some(msg) = args.get(pos + 1) {
            oneshot_chat(msg).await?;
            return Ok(());
        }
        eprintln!("Usage: mimo -c \"<message>\"");
        return Ok(());
    }

    interactive_cli().await?;

    Ok(())
}

async fn debug_repl() -> Result<()> {
    println!("🔧 MiMo Debug REPL Mode");
    println!("======================\n");
    println!("Commands:");
    println!("  :exit, :quit     - Exit the REPL");
    println!("  :clear           - Clear screen");
    println!("  :tools           - List all tools");
    println!("  :session         - Show current session");
    println!("  :chat <msg>      - Send message to MiMo (shorthand: just type)");
    println!("  :tool <name> <args> - Call tool directly");
    println!();
    println!("Or just type your message and press Enter to chat with MiMo.\n");

    let api_key = get_api_key()?;

    let provider = MimoProvider::new(api_key.clone());

    let health_ok = provider.health_check().await;
    if health_ok {
        println!("✓ MiMo API connection successful\n");
    } else {
        println!("✗ MiMo API connection failed\n");
        return Ok(());
    }

    let storage_path = dirs::home_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join(".mimo");

    std::fs::create_dir_all(&storage_path.join("sessions"))?;

    let session_manager = Arc::new(RefCell::new(SessionManager::new(storage_path.clone())));
    let tool_registry = Arc::new(ToolRegistry::new());
    let mut memory_manager = MemoryManager::new(&storage_path);
    memory_manager.take_snapshot();
    let recall_archive = Some(RecallArchive::new(storage_path.join("sessions")));
    let tool_runner = Arc::new(ToolRunner::new(memory_manager, recall_archive));

    let mut session = session_manager.borrow_mut().create_session();
    println!("📝 Session: {}\n", session.id);

    let tools = build_tools_from_registry(&tool_registry);

    loop {
        print!("[debug]> ");
        let _ = io::stdout().flush();

        let mut input = String::new();
        if io::stdin().read_line(&mut input).is_err() {
            break;
        }
        let input = input.trim();

        if input.is_empty() {
            continue;
        }

        if input == ":exit" || input == ":quit" {
            println!("Goodbye!");
            break;
        }

        if input == ":clear" {
            print!("\x1B[2J\x1B[1;1H");
            continue;
        }

        if input == ":tools" {
            println!("\nAvailable tools:");
            for category in tool_registry.categories() {
                print!("  {}: ", category);
                if let Some(tool_list) = tool_registry.list_by_category(category) {
                    let names: Vec<_> = tool_list.iter().map(|t| t.name.as_str()).collect();
                    println!("{}", names.join(", "));
                }
            }
            println!();
            continue;
        }

        if input == ":session" {
            println!("\n📝 Session ID: {}", session.id);
            println!("📂 Storage: {:?}\n", storage_path.join("sessions"));
            continue;
        }

        if input.starts_with(":tool ") {
            let parts: Vec<&str> = input.splitn(3, ' ').collect();
            if parts.len() >= 2 {
                let tool_name = parts[1];
                let tool_args = parts.get(2).unwrap_or(&"{}");
                println!(
                    "\n🔧 Calling tool: {} with args: {}\n",
                    tool_name, tool_args
                );

                if let Some(descriptor) = tool_registry.get(tool_name) {
                    let args: serde_json::Value =
                        serde_json::from_str(tool_args).unwrap_or(serde_json::json!({}));
                    let call = CoreToolCall {
                        id: "debug-call".to_string(),
                        name: tool_name.to_string(),
                        arguments: args,
                    };
                    let result = tool_runner.execute(call, descriptor).await;
                    match result {
                        CoreToolResult::Success(s) => println!("✅ Success:\n{}\n", s),
                        CoreToolResult::Error(e) => {
                            println!("❌ Error [{}]: {}\n", e.code, e.message)
                        }
                    }
                } else {
                    println!("❌ Unknown tool: {}\n", tool_name);
                }
            }
            continue;
        }

        if input.starts_with(":chat ") {
            let msg = input.trim_start_matches(":chat ");
            process_message(
                msg,
                &provider,
                &tools,
                &session_manager,
                &mut session,
                &tool_registry,
                &tool_runner,
            )
            .await?;
        } else {
            process_message(
                input,
                &provider,
                &tools,
                &session_manager,
                &mut session,
                &tool_registry,
                &tool_runner,
            )
            .await?;
        }
    }

    Ok(())
}

async fn process_message(
    input: &str,
    provider: &MimoProvider,
    tools: &[ToolDefinition],
    session_manager: &Arc<RefCell<SessionManager>>,
    session: &mut Session,
    tool_registry: &Arc<ToolRegistry>,
    tool_runner: &Arc<ToolRunner>,
) -> Result<()> {
    session.add_message(MessageRole::User, input.to_string());

    let mut messages: Vec<ProviderMessage> = Vec::new();

    if let Some(memory_block) = tool_runner.memory_manager().get_system_prompt_block() {
        let system_content = format!(
            "你是一个智能编程助手 MiMo，基于 MiMo V2.5 Pro 模型。\n\
             你可以使用工具来完成用户的请求。\n\
             你的记忆在下方显示，这些记忆在会话之间持久保存。\n\
             使用 remember 工具保存新记忆，使用 memory_replace 更新条目，使用 memory_remove 删除条目。\n\n{}",
            memory_block
        );
        messages.push(ProviderMessage {
            role: ProviderMessageRole::System,
            content: system_content,
            name: None,
            tool_calls: None,
        });
    }

    for m in &session.messages {
        messages.push(ProviderMessage {
            role: convert_role(&m.role),
            content: m.content.clone(),
            name: None,
            tool_calls: None,
        });
    }

    let options = ChatOptions {
        temperature: Some(0.7),
        max_tokens: Some(4096),
        top_p: None,
        thinking: Some(mimo_providers::provider::ThinkingOptions {
            enabled: true,
            effort: Some("medium".to_string()),
        }),
        tools: Some(tools.to_vec()),
    };

    print!("\n[MiMo] ");
    let _ = io::stdout().flush();

    let response = match provider
        .chat_completions(messages.clone(), DEFAULT_MODEL, options)
        .await
    {
        Ok(r) => r,
        Err(e) => {
            println!("\n❌ Error: {}", e);
            return Ok(());
        }
    };

    let assistant_message = &response
        .choices
        .first()
        .ok_or_else(|| anyhow::anyhow!("No response choices"))?
        .message;

    println!("{}", assistant_message.content);

    session.add_message(MessageRole::Assistant, assistant_message.content.clone());

    if let Some(tool_calls) = &assistant_message.tool_calls {
        if !tool_calls.is_empty() {
            for tool_call in tool_calls {
                let call_id = &tool_call.id;
                let tool_name = &tool_call.name;
                let args = &tool_call.arguments;

                print!("\n\n[🔧 Tool: {}] ", tool_name);
                let _ = io::stdout().flush();

                let result = if let Some(descriptor) = tool_registry.get(tool_name) {
                    let core_call = CoreToolCall {
                        id: call_id.clone(),
                        name: tool_name.clone(),
                        arguments: serde_json::to_value(args)?,
                    };
                    tool_runner.execute(core_call, descriptor).await
                } else {
                    CoreToolResult::Error(mimo_core::tools::ToolError::new(
                        "unknown_tool",
                        format!("Tool '{}' not found", tool_name),
                    ))
                };

                let result_content = match &result {
                    CoreToolResult::Success(s) => s.clone(),
                    CoreToolResult::Error(e) => format!("Error: {} - {}", e.code, e.message),
                };

                println!("{}", &result_content[..result_content.len().min(500)]);
                if result_content.len() > 500 {
                    println!("... (truncated, {} chars total)", result_content.len());
                }

                session.add_message(
                    MessageRole::Tool,
                    format!("[{}] {}", tool_name, result_content),
                );
            }
        }
    }

    session_manager.borrow_mut().save_session(session)?;
    println!();

    Ok(())
}

async fn interactive_cli() -> Result<()> {
    println!("MiMo-TUI v0.1.0");
    println!("=============\n");

    let storage_path = dirs::home_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join(".mimo");

    std::fs::create_dir_all(&storage_path.join("sessions"))?;

    let mut memory_manager = MemoryManager::new(&storage_path);
    memory_manager.take_snapshot();
    let recall_archive = Some(RecallArchive::new(storage_path.join("sessions")));

    let session_manager = Arc::new(RefCell::new(SessionManager::new(storage_path.clone())));
    let tool_registry = Arc::new(ToolRegistry::new());
    let tool_runner = Arc::new(ToolRunner::new(memory_manager, recall_archive));
    let _subagent_manager = Arc::new(SubAgentManager::new());

    let api_key = get_api_key()?;

    let provider = MimoProvider::new(api_key);

    let health_ok = provider.health_check().await;
    if health_ok {
        println!("✓ MiMo API connection successful");
    } else {
        println!("✗ MiMo API connection failed");
        return Ok(());
    }

    let mut session = session_manager.borrow_mut().create_session();
    println!("Session: {}\n", session.id);

    let tools = build_tools_from_registry(&tool_registry);
    let mut turn_count = 0;

    loop {
        print!("\n[You]> ");
        io::stdout().flush()?;

        let mut input = String::new();
        if io::stdin().read_line(&mut input).is_err() {
            break;
        }
        let input = input.trim();
        if input.is_empty() {
            continue;
        }

        if input == "/exit" || input == "/quit" {
            println!("Goodbye!");
            break;
        }

        if input == "/help" {
            print_help();
            continue;
        }

        if input == "/tools" {
            list_tools(&tool_registry);
            continue;
        }

        if input == "/sessions" {
            list_sessions(&session_manager);
            continue;
        }

        if input.starts_with("/session ") {
            let id = input.trim_start_matches("/session ").trim();
            if let Some(_s) = session_manager.borrow().get_session(id) {
                println!("Switched to session: {}", id);
            } else {
                println!("Session not found: {}", id);
            }
            continue;
        }

        session.add_message(MessageRole::User, input.to_string());
        session_manager.borrow_mut().save_session(&session)?;

        let mut messages: Vec<ProviderMessage> = Vec::new();

        if let Some(memory_block) = tool_runner.memory_manager().get_system_prompt_block() {
            let system_content = format!(
                "你是一个智能编程助手 MiMo，基于 MiMo V2.5 Pro 模型。\n\
                 你可以使用工具来完成用户的请求。\n\
                 你的记忆在下方显示，这些记忆在会话之间持久保存。\n\
                 使用 remember 工具保存新记忆，使用 memory_replace 更新条目，使用 memory_remove 删除条目。\n\n{}",
                memory_block
            );
            messages.push(ProviderMessage {
                role: ProviderMessageRole::System,
                content: system_content,
                name: None,
                tool_calls: None,
            });
        }

        for m in &session.messages {
            messages.push(ProviderMessage {
                role: convert_role(&m.role),
                content: m.content.clone(),
                name: None,
                tool_calls: None,
            });
        }

        let options = ChatOptions {
            temperature: Some(0.7),
            max_tokens: Some(4096),
            top_p: None,
            thinking: Some(mimo_providers::provider::ThinkingOptions {
                enabled: true,
                effort: Some("medium".to_string()),
            }),
            tools: Some(tools.clone()),
        };

        print!("\n[MiMo] ");
        io::stdout().flush()?;

        let response = match provider
            .chat_completions(messages.clone(), DEFAULT_MODEL, options)
            .await
        {
            Ok(r) => r,
            Err(e) => {
                println!("\nError: {}", e);
                continue;
            }
        };

        let assistant_message = &response
            .choices
            .first()
            .ok_or_else(|| anyhow::anyhow!("No response choices"))?
            .message;

        print!("{}", assistant_message.content);
        let _ = io::stdout().flush();

        session.add_message(MessageRole::Assistant, assistant_message.content.clone());

        if let Some(tool_calls) = &assistant_message.tool_calls {
            if !tool_calls.is_empty() {
                for tool_call in tool_calls {
                    let call_id = &tool_call.id;
                    let tool_name = &tool_call.name;
                    let args = &tool_call.arguments;

                    print!("\n\n[Tool: {}] ", tool_name);
                    let _ = io::stdout().flush();

                    let result = if let Some(descriptor) = tool_registry.get(tool_name) {
                        let core_call = CoreToolCall {
                            id: call_id.clone(),
                            name: tool_name.clone(),
                            arguments: serde_json::to_value(args)?,
                        };
                        tool_runner.execute(core_call, descriptor).await
                    } else {
                        CoreToolResult::Error(mimo_core::tools::ToolError::new(
                            "unknown_tool",
                            format!("Tool '{}' not found in registry", tool_name),
                        ))
                    };

                    let result_content = match result {
                        CoreToolResult::Success(s) => s,
                        CoreToolResult::Error(e) => format!("Error: {} - {}", e.code, e.message),
                    };

                    println!("{}", &result_content[..result_content.len().min(500)]);
                    if result_content.len() > 500 {
                        println!("... (truncated, {} chars total)", result_content.len());
                    }

                    session.add_message(
                        MessageRole::Tool,
                        format!("[{}] {}", tool_name, result_content),
                    );
                }

                session_manager.borrow_mut().save_session(&session)?;
                turn_count += 1;

                if turn_count >= MAX_TURN_DEPTH {
                    println!("\n\n[Max turn depth reached. Starting new session...]");
                    session = session_manager.borrow_mut().create_session();
                    turn_count = 0;
                    println!("New session: {}", session.id);
                }
            }
        } else {
            session_manager.borrow_mut().save_session(&session)?;
        }

        println!("\n");
    }

    println!("\nSessions saved to: {:?}", storage_path.join("sessions"));
    Ok(())
}

fn print_usage() {
    println!(
        r#"MiMo-TUI v0.1.0 - Intelligent Programming Assistant

Usage:
  mimo                    Launch interactive CLI (default)
  mimo [flags]            Launch with flags
  mimo <subcommand>       Launch with subcommand

Flags:
  -s, --start-tui         Launch TUI interface
  -d, --debug, --repl     Launch debug REPL mode
  -c, --chat <message>    One-shot chat (non-interactive)
  -h, --help              Show this help message
  -v, --version           Show version

Subcommands:
  tui                     Same as -s / --start-tui
  repl                    Same as -d / --debug
  chat <message>          Same as -c <message>

Examples:
  mimo                    Start interactive chat
  mimo -s                 Start TUI from any directory
  mimo -c "explain this code"   Ask a question without interactive mode
  mimo tui                Start TUI (subcommand style)
  mimo repl               Start debug REPL mode
"#
    );
}

async fn oneshot_chat(message: &str) -> Result<()> {
    let api_key = get_api_key()?;
    let provider = MimoProvider::new(api_key);

    let messages = vec![ProviderMessage {
        role: ProviderMessageRole::User,
        content: message.to_string(),
        name: None,
        tool_calls: None,
    }];

    let options = ChatOptions {
        temperature: Some(0.7),
        max_tokens: Some(4096),
        top_p: None,
        thinking: Some(mimo_providers::provider::ThinkingOptions {
            enabled: true,
            effort: Some("medium".to_string()),
        }),
        tools: None,
    };

    let response = provider
        .chat_completions(messages, DEFAULT_MODEL, options)
        .await?;

    let content = &response
        .choices
        .first()
        .ok_or_else(|| anyhow::anyhow!("No response choices"))?
        .message
        .content;

    println!("{}", content);

    Ok(())
}

fn print_help() {
    println!(
        r#"
Available commands:
  /exit, /quit    Exit the application
  /help           Show this help message
  /tools          List available tools
  /sessions       List all sessions
  /session <id>   Switch to a session

Conversation:
  Type your message and press Enter to chat with MiMo.
  MiMo can use tools to help answer your questions.
"#
    );
}

fn list_tools(registry: &ToolRegistry) {
    println!("\nAvailable tools:");
    for category in registry.categories() {
        print!("  {}: ", category);
        if let Some(tools) = registry.list_by_category(category) {
            let names: Vec<_> = tools.iter().map(|t| t.name.as_str()).collect();
            println!("{}", names.join(", "));
        }
    }
    println!();
}

fn list_sessions(manager: &RefCell<SessionManager>) {
    println!("\nSessions:");
    for session in manager.borrow().list_sessions() {
        println!("  {} - {}", session.id, session.preview());
    }
    println!();
}
