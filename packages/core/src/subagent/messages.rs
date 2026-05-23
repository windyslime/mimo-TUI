use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SubAgentMessage {
    TaskAssignment(TaskAssignment),
    ProgressUpdate(ProgressUpdate),
    ResultNotification(ResultNotification),
    Cancellation { agent_id: String },
    InterruptSignal { agent_id: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskAssignment {
    pub agent_id: String,
    pub role: String,
    pub task: String,
    pub model: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProgressUpdate {
    pub agent_id: String,
    pub progress: u8,
    pub message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResultNotification {
    pub agent_id: String,
    pub summary: String,
    pub changes: Vec<String>,
    pub evidence: Vec<String>,
    pub risks: Vec<String>,
    pub blockers: Vec<String>,
}
