use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubAgent {
    pub id: String,
    pub role: SubAgentRole,
    pub task: String,
    pub status: SubAgentStatus,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub result: Option<SubAgentResult>,
    pub progress: u8,
    pub progress_message: Option<String>,
    pub stream_content: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum SubAgentRole {
    General,
    Explore,
    Plan,
    Review,
    Implementer,
    Verifier,
    Custom,
}

impl SubAgentRole {
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "general" => Some(Self::General),
            "explore" => Some(Self::Explore),
            "plan" => Some(Self::Plan),
            "review" => Some(Self::Review),
            "implementer" => Some(Self::Implementer),
            "verifier" => Some(Self::Verifier),
            "custom" => Some(Self::Custom),
            _ => None,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            SubAgentRole::General => "general",
            SubAgentRole::Explore => "explore",
            SubAgentRole::Plan => "plan",
            SubAgentRole::Review => "review",
            SubAgentRole::Implementer => "implementer",
            SubAgentRole::Verifier => "verifier",
            SubAgentRole::Custom => "custom",
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            SubAgentRole::General => "Full access, all tools available",
            SubAgentRole::Explore => "Read-only code exploration and research",
            SubAgentRole::Plan => "Task planning and decomposition",
            SubAgentRole::Review => "Code review and analysis",
            SubAgentRole::Implementer => "Code implementation and modification",
            SubAgentRole::Verifier => "Test running and verification",
            SubAgentRole::Custom => "Configurable access, whitelist-based",
        }
    }

    pub fn system_prompt(&self) -> &'static str {
        match self {
            SubAgentRole::General => {
                "You are a general-purpose sub-agent. You have full access to all tools. \
                 Complete the assigned task thoroughly and return a structured result."
            }
            SubAgentRole::Explore => {
                "You are a code exploration sub-agent. Use read-only tools (file_read, grep, glob, git_status) \
                 to search and analyze the codebase. Do NOT modify any files. \
                 Return findings with file paths and line references."
            }
            SubAgentRole::Plan => {
                "You are a planning sub-agent. Analyze the task and create a detailed step-by-step plan. \
                 You may use read-only tools to understand the codebase, but focus on producing a clear plan."
            }
            SubAgentRole::Review => {
                "You are a code review sub-agent. Examine code for bugs, security issues, \
                 and design problems. Use read-only tools only. Return a structured review."
            }
            SubAgentRole::Implementer => {
                "You are an implementation sub-agent. Write and modify code to complete the assigned task. \
                 Use all available tools to read, write, search, and execute code."
            }
            SubAgentRole::Verifier => {
                "You are a verification sub-agent. Run tests, check build status, and verify correctness. \
                 Return test results and any issues found."
            }
            SubAgentRole::Custom => {
                "You are a custom sub-agent with limited tool access. Complete the assigned task \
                 using only the tools available to you."
            }
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum SubAgentStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Interrupted,
    Cancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubAgentResult {
    pub summary: String,
    pub changes: Vec<String>,
    pub evidence: Vec<String>,
    pub risks: Vec<String>,
    pub blockers: Vec<String>,
}

impl SubAgent {
    pub fn new(role: SubAgentRole, task: String) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4().to_string(),
            role,
            task,
            status: SubAgentStatus::Pending,
            created_at: now,
            updated_at: now,
            result: None,
            progress: 0,
            progress_message: None,
            stream_content: String::new(),
        }
    }

    pub fn start(&mut self) {
        self.status = SubAgentStatus::Running;
        self.progress = 5;
        self.updated_at = Utc::now();
    }

    pub fn update_progress(&mut self, progress: u8, message: Option<String>) {
        self.progress = progress.min(99);
        self.progress_message = message;
        self.updated_at = Utc::now();
    }

    pub fn append_content(&mut self, content: &str) {
        self.stream_content.push_str(content);
        self.updated_at = Utc::now();
    }

    pub fn complete(&mut self, result: SubAgentResult) {
        self.status = SubAgentStatus::Completed;
        self.progress = 100;
        self.result = Some(result);
        self.updated_at = Utc::now();
    }

    pub fn fail(&mut self, error: String) {
        self.status = SubAgentStatus::Failed;
        self.progress_message = Some(error);
        self.updated_at = Utc::now();
    }

    pub fn interrupt(&mut self) {
        self.status = SubAgentStatus::Interrupted;
        self.updated_at = Utc::now();
    }

    pub fn cancel(&mut self) {
        self.status = SubAgentStatus::Cancelled;
        self.updated_at = Utc::now();
    }

    pub fn resume(&mut self) {
        self.status = SubAgentStatus::Running;
        self.updated_at = Utc::now();
    }
}
