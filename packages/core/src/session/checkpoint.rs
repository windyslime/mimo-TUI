use super::Session;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Checkpoint {
    pub id: String,
    pub session_id: String,
    pub created_at: DateTime<Utc>,
    pub reason: String,
    pub session_state: SessionState,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionState {
    pub messages: Vec<super::Message>,
}

impl Checkpoint {
    pub fn new(session: Session, reason: String) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            session_id: session.id.clone(),
            created_at: Utc::now(),
            reason,
            session_state: SessionState {
                messages: session.messages,
            },
        }
    }

    pub fn restore(&self) -> Session {
        Session {
            id: self.session_id.clone(),
            created_at: self.created_at,
            updated_at: Utc::now(),
            messages: self.session_state.messages.clone(),
            metadata: Default::default(),
        }
    }
}
