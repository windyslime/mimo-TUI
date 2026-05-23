use crate::memory::MemoryManager;
use crate::session::SessionManager;
use anyhow::Result;
use std::sync::Arc;

pub struct TurnLoop {}

impl TurnLoop {
    pub fn new() -> Self {
        Self {}
    }

    pub async fn execute(
        &mut self,
        session_id: &str,
        _session_manager: &Arc<SessionManager>,
        memory_manager: &MemoryManager,
    ) -> Result<()> {
        tracing::info!("Executing turn for session: {}", session_id);

        let mem_usage = memory_manager.memory_usage(crate::memory::MemoryTarget::Memory);
        tracing::info!(
            "Memory usage: {}/{} chars ({}%)",
            mem_usage.chars,
            mem_usage.limit_chars,
            mem_usage.pct
        );

        Ok(())
    }
}

impl Default for TurnLoop {
    fn default() -> Self {
        Self::new()
    }
}
