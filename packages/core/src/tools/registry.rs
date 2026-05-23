use super::types::{ToolDescriptor, ToolHandler};
use std::collections::HashMap;

pub struct ToolRegistry {
    tools: HashMap<String, ToolDescriptor>,
    categories: HashMap<String, Vec<String>>,
}

impl ToolRegistry {
    pub fn new() -> Self {
        let mut registry = Self {
            tools: HashMap::new(),
            categories: HashMap::new(),
        };
        registry.register_default_tools();
        registry
    }

    fn register_default_tools(&mut self) {
        self.register(ToolDescriptor {
            name: "file_read".to_string(),
            description: "Read contents of a file".to_string(),
            input_schema: HashMap::new(),
            output_schema: None,
            required_permissions: vec![],
            handler: ToolHandler::FileRead,
        });

        self.register(ToolDescriptor {
            name: "file_write".to_string(),
            description: "Write content to a file".to_string(),
            input_schema: HashMap::new(),
            output_schema: None,
            required_permissions: vec!["file_write".to_string()],
            handler: ToolHandler::FileWrite,
        });

        self.register(ToolDescriptor {
            name: "shell".to_string(),
            description: "Execute shell commands".to_string(),
            input_schema: HashMap::new(),
            output_schema: None,
            required_permissions: vec!["shell".to_string()],
            handler: ToolHandler::Shell,
        });

        self.register(ToolDescriptor {
            name: "grep".to_string(),
            description: "Search for patterns in files".to_string(),
            input_schema: HashMap::new(),
            output_schema: None,
            required_permissions: vec![],
            handler: ToolHandler::Grep,
        });

        self.register(ToolDescriptor {
            name: "glob".to_string(),
            description: "Find files by pattern".to_string(),
            input_schema: HashMap::new(),
            output_schema: None,
            required_permissions: vec![],
            handler: ToolHandler::Glob,
        });

        self.register(ToolDescriptor {
            name: "git_status".to_string(),
            description: "Show git repository status".to_string(),
            input_schema: HashMap::new(),
            output_schema: None,
            required_permissions: vec![],
            handler: ToolHandler::Git,
        });

        self.register(ToolDescriptor {
            name: "web_fetch".to_string(),
            description: "Fetch content from a URL".to_string(),
            input_schema: HashMap::new(),
            output_schema: None,
            required_permissions: vec!["network".to_string()],
            handler: ToolHandler::WebFetch,
        });

        self.register(ToolDescriptor {
            name: "web_search".to_string(),
            description: "Search the web".to_string(),
            input_schema: HashMap::new(),
            output_schema: None,
            required_permissions: vec!["network".to_string()],
            handler: ToolHandler::WebSearch,
        });

        self.register(ToolDescriptor {
            name: "remember".to_string(),
            description: "Save a note to persistent memory. Use target='memory' for agent notes (environment, project facts, learnings) or target='user' for user profile (preferences, communication style, habits). Do NOT use for secrets.".to_string(),
            input_schema: HashMap::new(),
            output_schema: None,
            required_permissions: vec![],
            handler: ToolHandler::Remember,
        });

        self.register(ToolDescriptor {
            name: "memory_replace".to_string(),
            description: "Replace an existing memory entry. Uses substring matching via old_text to locate the entry. If multiple entries match, an error is returned — provide a more specific old_text.".to_string(),
            input_schema: HashMap::new(),
            output_schema: None,
            required_permissions: vec!["memory_write".to_string()],
            handler: ToolHandler::MemoryReplace,
        });

        self.register(ToolDescriptor {
            name: "memory_remove".to_string(),
            description: "Remove a memory entry. Uses substring matching via old_text to locate the entry. If multiple entries match, an error is returned — provide a more specific old_text.".to_string(),
            input_schema: HashMap::new(),
            output_schema: None,
            required_permissions: vec!["memory_write".to_string()],
            handler: ToolHandler::MemoryRemove,
        });

        self.register(ToolDescriptor {
            name: "recall_archive".to_string(),
            description: "Search prior conversation archives for relevant context. Use when you need to recall past discussions or decisions.".to_string(),
            input_schema: HashMap::new(),
            output_schema: None,
            required_permissions: vec![],
            handler: ToolHandler::RecallArchive,
        });

        self.register(ToolDescriptor {
            name: "agent_spawn".to_string(),
            description: "Spawn a new sub-agent to handle a specific task. The sub-agent runs asynchronously and can use tools independently. Use this to delegate complex sub-tasks. Available roles: general (full access), explore (read-only search), plan (planning only), review (code review), implementer (write code), verifier (run tests), custom (limited access). Returns the agent_id for tracking.".to_string(),
            input_schema: HashMap::new(),
            output_schema: None,
            required_permissions: vec!["agent".to_string()],
            handler: ToolHandler::AgentSpawn,
        });

        self.register(ToolDescriptor {
            name: "agent_wait".to_string(),
            description: "Wait for a sub-agent to complete its task. Blocks until the agent finishes or timeout is reached. Returns the agent's result including summary, changes, evidence, risks, and blockers.".to_string(),
            input_schema: HashMap::new(),
            output_schema: None,
            required_permissions: vec!["agent".to_string()],
            handler: ToolHandler::AgentWait,
        });

        self.register(ToolDescriptor {
            name: "agent_result".to_string(),
            description: "Get the current result of a sub-agent without waiting. Returns None if the agent hasn't completed yet, or the agent's result if it has.".to_string(),
            input_schema: HashMap::new(),
            output_schema: None,
            required_permissions: vec!["agent".to_string()],
            handler: ToolHandler::AgentResult,
        });

        self.register(ToolDescriptor {
            name: "agent_cancel".to_string(),
            description: "Cancel a running sub-agent. The agent will be marked as cancelled and any waiters will be notified.".to_string(),
            input_schema: HashMap::new(),
            output_schema: None,
            required_permissions: vec!["agent".to_string()],
            handler: ToolHandler::AgentCancel,
        });

        self.register(ToolDescriptor {
            name: "agent_list".to_string(),
            description: "List all sub-agents and their current status. Returns agent IDs, roles, status (pending/running/completed/failed/cancelled), and progress information.".to_string(),
            input_schema: HashMap::new(),
            output_schema: None,
            required_permissions: vec!["agent".to_string()],
            handler: ToolHandler::AgentList,
        });

        self.categories.insert(
            "file".to_string(),
            vec![
                "file_read".to_string(),
                "file_write".to_string(),
                "glob".to_string(),
            ],
        );
        self.categories.insert(
            "search".to_string(),
            vec!["grep".to_string(), "glob".to_string()],
        );
        self.categories
            .insert("shell".to_string(), vec!["shell".to_string()]);
        self.categories
            .insert("git".to_string(), vec!["git_status".to_string()]);
        self.categories.insert(
            "web".to_string(),
            vec!["web_fetch".to_string(), "web_search".to_string()],
        );
        self.categories.insert(
            "memory".to_string(),
            vec![
                "remember".to_string(),
                "memory_replace".to_string(),
                "memory_remove".to_string(),
                "recall_archive".to_string(),
            ],
        );
        self.categories.insert(
            "agent".to_string(),
            vec![
                "agent_spawn".to_string(),
                "agent_wait".to_string(),
                "agent_result".to_string(),
                "agent_cancel".to_string(),
                "agent_list".to_string(),
            ],
        );
    }

    pub fn register(&mut self, tool: ToolDescriptor) {
        self.tools.insert(tool.name.clone(), tool);
    }

    pub fn get(&self, name: &str) -> Option<&ToolDescriptor> {
        self.tools.get(name)
    }

    pub fn list(&self) -> Vec<&ToolDescriptor> {
        self.tools.values().collect()
    }

    pub fn list_by_category(&self, category: &str) -> Option<Vec<&ToolDescriptor>> {
        self.categories
            .get(category)
            .map(|names| names.iter().filter_map(|n| self.tools.get(n)).collect())
    }

    pub fn categories(&self) -> Vec<&str> {
        self.categories.keys().map(|s| s.as_str()).collect()
    }

    pub fn tool_names(&self) -> Vec<&String> {
        self.tools.keys().collect()
    }
}

impl Default for ToolRegistry {
    fn default() -> Self {
        Self::new()
    }
}
