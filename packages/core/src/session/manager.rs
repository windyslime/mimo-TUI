use super::{Checkpoint, Message, MessageRole, Session};
use anyhow::Result;
use std::collections::HashMap;
use std::path::PathBuf;

pub struct SessionManager {
    sessions: HashMap<String, Session>,
    storage_path: PathBuf,
}

impl SessionManager {
    pub fn new(storage_path: PathBuf) -> Self {
        Self {
            sessions: HashMap::new(),
            storage_path,
        }
    }

    pub fn create_session(&mut self) -> Session {
        let session = Session::new();
        self.sessions.insert(session.id.clone(), session.clone());
        session
    }

    pub fn get_session(&self, id: &str) -> Option<&Session> {
        self.sessions.get(id)
    }

    pub fn get_session_mut(&mut self, id: &str) -> Option<&mut Session> {
        self.sessions.get_mut(id)
    }

    pub fn list_sessions(&self) -> Vec<&Session> {
        self.sessions.values().collect()
    }

    pub fn delete_session(&mut self, id: &str) -> Option<Session> {
        self.sessions.remove(id)
    }

    pub fn add_message(
        &mut self,
        session_id: &str,
        role: MessageRole,
        content: String,
    ) -> Option<&Message> {
        self.sessions
            .get_mut(session_id)
            .map(|s| s.add_message(role, content))
    }

    pub fn save_session(&self, session: &Session) -> Result<()> {
        let path = self.session_path(&session.id);
        let json = serde_json::to_string_pretty(session)?;
        std::fs::write(path, json)?;
        Ok(())
    }

    pub fn load_session(&mut self, id: &str) -> Result<Option<Session>> {
        let path = self.session_path(id);
        if !path.exists() {
            return Ok(None);
        }
        let content = std::fs::read_to_string(path)?;
        let session: Session = serde_json::from_str(&content)?;
        self.sessions.insert(id.to_string(), session.clone());
        Ok(Some(session))
    }

    pub fn create_checkpoint(&self, session_id: &str, reason: &str) -> Result<Checkpoint> {
        let session = self
            .get_session(session_id)
            .ok_or_else(|| anyhow::anyhow!("Session not found"))?;
        let checkpoint = Checkpoint::new(session.clone(), reason.to_string());
        self.save_checkpoint(&checkpoint)?;
        Ok(checkpoint)
    }

    pub fn save_checkpoint(&self, checkpoint: &Checkpoint) -> Result<()> {
        let path = self.checkpoint_path(&checkpoint.session_id, &checkpoint.id);
        let json = serde_json::to_string_pretty(checkpoint)?;
        std::fs::write(path, json)?;
        Ok(())
    }

    fn session_path(&self, id: &str) -> PathBuf {
        self.storage_path
            .join("sessions")
            .join(format!("{}.json", id))
    }

    fn checkpoint_path(&self, session_id: &str, checkpoint_id: &str) -> PathBuf {
        self.storage_path
            .join("sessions")
            .join("checkpoints")
            .join(format!("{}.{}.ckpt", session_id, checkpoint_id))
    }
}
