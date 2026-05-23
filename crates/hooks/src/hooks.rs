use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum HookEvent {
    ResponseStart {
        response_id: String,
    },
    ResponseEnd {
        response_id: String,
    },
    ResponseDelta {
        response_id: String,
        delta: String,
    },
    ToolLifecycle {
        response_id: String,
        tool_name: String,
        phase: String,
        payload: serde_json::Value,
    },
    ApprovalLifecycle {
        approval_id: String,
        phase: String,
        reason: Option<String>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum HookPhase {
    PreExecution,
    PostExecution,
    OnError,
}

pub struct HookDispatcher {
    handlers: Arc<RwLock<Vec<Box<dyn HookHandler + Send + Sync>>>>,
}

#[async_trait::async_trait]
pub trait HookHandler: Send + Sync {
    async fn handle(&self, event: HookEvent);
}

impl HookDispatcher {
    pub fn new() -> Self {
        Self {
            handlers: Arc::new(RwLock::new(Vec::new())),
        }
    }

    pub async fn register<H: HookHandler + Send + Sync + 'static>(&self, handler: H) {
        let mut handlers = self.handlers.write().await;
        handlers.push(Box::new(handler));
    }

    pub async fn emit(&self, event: HookEvent) {
        let handlers = self.handlers.read().await;
        for handler in handlers.iter() {
            handler.handle(event.clone()).await;
        }
    }
}

impl Default for HookDispatcher {
    fn default() -> Self {
        Self::new()
    }
}
