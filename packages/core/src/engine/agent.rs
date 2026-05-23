use super::context::ContextManager;
use super::turn_loop::TurnLoop;
use crate::memory::MemoryManager;
use crate::session::SessionManager;
use crate::subagent::SubAgentManager;
use crate::tools::ToolRegistry;
use anyhow::Result;
use std::sync::Arc;

pub struct AgentEngine {
    session_manager: Arc<SessionManager>,
    _tool_registry: Arc<ToolRegistry>,
    subagent_manager: Arc<SubAgentManager>,
    _context_manager: ContextManager,
    turn_loop: TurnLoop,
    memory_manager: MemoryManager,
}

impl AgentEngine {
    pub fn new(
        session_manager: Arc<SessionManager>,
        tool_registry: Arc<ToolRegistry>,
        subagent_manager: Arc<SubAgentManager>,
        memory_manager: MemoryManager,
    ) -> Self {
        let mut mm = memory_manager;
        mm.take_snapshot();
        Self {
            session_manager,
            _tool_registry: tool_registry,
            subagent_manager,
            _context_manager: ContextManager::new(),
            turn_loop: TurnLoop::new(),
            memory_manager: mm,
        }
    }

    pub async fn run_turn(&mut self, session_id: &str) -> Result<()> {
        self.turn_loop
            .execute(session_id, &self.session_manager, &self.memory_manager)
            .await
    }

    pub async fn shutdown(&mut self) {
        self.subagent_manager.shutdown().await;
    }

    pub fn get_memory_block(&self) -> Option<String> {
        self.memory_manager.get_system_prompt_block()
    }

    pub fn memory_manager(&self) -> &MemoryManager {
        &self.memory_manager
    }
}
