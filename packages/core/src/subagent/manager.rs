use super::subagent::{SubAgent, SubAgentResult, SubAgentRole, SubAgentStatus};
use anyhow::Result;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{Notify, RwLock, Semaphore};

const DEFAULT_MAX_CONCURRENT: usize = 10;
const MAX_CONCURRENT_LIMIT: usize = 20;

pub struct SubAgentManager {
    agents: Arc<RwLock<HashMap<String, SubAgent>>>,
    completion_notifiers: Arc<RwLock<HashMap<String, Arc<Notify>>>>,
    max_concurrent: usize,
    semaphore: Arc<Semaphore>,
}

impl SubAgentManager {
    pub fn new() -> Self {
        let max = DEFAULT_MAX_CONCURRENT;
        Self {
            agents: Arc::new(RwLock::new(HashMap::new())),
            completion_notifiers: Arc::new(RwLock::new(HashMap::new())),
            max_concurrent: max,
            semaphore: Arc::new(Semaphore::new(max)),
        }
    }

    pub fn with_max_concurrent(max: usize) -> Self {
        let m = max.min(MAX_CONCURRENT_LIMIT);
        Self {
            agents: Arc::new(RwLock::new(HashMap::new())),
            completion_notifiers: Arc::new(RwLock::new(HashMap::new())),
            max_concurrent: m,
            semaphore: Arc::new(Semaphore::new(m)),
        }
    }

    pub fn semaphore(&self) -> Arc<Semaphore> {
        self.semaphore.clone()
    }

    pub fn max_concurrent(&self) -> usize {
        self.max_concurrent
    }

    pub async fn spawn(&self, role: SubAgentRole, task: String) -> String {
        let agent = SubAgent::new(role, task);
        let id = agent.id.clone();
        let agent_task = agent.task.clone();
        let notify = Arc::new(Notify::new());
        {
            let mut agents = self.agents.write().await;
            agents.insert(id.clone(), agent);
        }
        {
            let mut notifiers = self.completion_notifiers.write().await;
            notifiers.insert(id.clone(), notify);
        }
        tracing::info!("Spawned sub-agent {} ({:?}): {}", id, role, agent_task);
        id
    }

    pub async fn start(&self, id: &str) -> Result<()> {
        let mut agents = self.agents.write().await;
        if let Some(agent) = agents.get_mut(id) {
            agent.start();
            Ok(())
        } else {
            Err(anyhow::anyhow!("SubAgent {} not found", id))
        }
    }

    pub async fn update_progress(
        &self,
        id: &str,
        progress: u8,
        message: Option<String>,
    ) -> Result<()> {
        let mut agents = self.agents.write().await;
        if let Some(agent) = agents.get_mut(id) {
            agent.update_progress(progress, message);
            Ok(())
        } else {
            Err(anyhow::anyhow!("SubAgent {} not found", id))
        }
    }

    pub async fn append_content(&self, id: &str, content: &str) -> Result<()> {
        let mut agents = self.agents.write().await;
        if let Some(agent) = agents.get_mut(id) {
            agent.append_content(content);
            Ok(())
        } else {
            Err(anyhow::anyhow!("SubAgent {} not found", id))
        }
    }

    pub async fn wait(
        &self,
        id: &str,
        timeout_secs: Option<u64>,
    ) -> Result<Option<SubAgentResult>> {
        let notify = {
            let notifiers = self.completion_notifiers.read().await;
            notifiers.get(id).cloned()
        };

        let notify = match notify {
            Some(n) => n,
            None => return Err(anyhow::anyhow!("SubAgent {} not found", id)),
        };

        let notified = if let Some(secs) = timeout_secs {
            tokio::time::timeout(std::time::Duration::from_secs(secs), notify.notified())
                .await
                .is_ok()
        } else {
            notify.notified().await;
            true
        };

        if !notified {
            return Ok(None);
        }

        let agents = self.agents.read().await;
        if let Some(agent) = agents.get(id) {
            if agent.status == SubAgentStatus::Completed || agent.status == SubAgentStatus::Failed {
                Ok(agent.result.clone())
            } else {
                Ok(None)
            }
        } else {
            Err(anyhow::anyhow!("SubAgent {} not found", id))
        }
    }

    pub async fn complete_subagent(&self, id: &str, result: SubAgentResult) -> Result<()> {
        let mut agents = self.agents.write().await;
        if let Some(agent) = agents.get_mut(id) {
            agent.complete(result);
            self.semaphore.add_permits(1);
            drop(agents);
            let notifiers = self.completion_notifiers.read().await;
            if let Some(notify) = notifiers.get(id) {
                notify.notify_waiters();
            }
            Ok(())
        } else {
            Err(anyhow::anyhow!("SubAgent {} not found", id))
        }
    }

    pub async fn fail_subagent(&self, id: &str, error: String) -> Result<()> {
        let mut agents = self.agents.write().await;
        if let Some(agent) = agents.get_mut(id) {
            agent.fail(error);
            self.semaphore.add_permits(1);
            drop(agents);
            let notifiers = self.completion_notifiers.read().await;
            if let Some(notify) = notifiers.get(id) {
                notify.notify_waiters();
            }
            Ok(())
        } else {
            Err(anyhow::anyhow!("SubAgent {} not found", id))
        }
    }

    pub async fn result(&self, id: &str) -> Result<Option<SubAgentResult>> {
        let agents = self.agents.read().await;
        Ok(agents.get(id).and_then(|a| a.result.clone()))
    }

    pub async fn cancel(&self, id: &str) -> Result<()> {
        let mut agents = self.agents.write().await;
        if let Some(agent) = agents.get_mut(id) {
            let was_running = agent.status == SubAgentStatus::Running;
            agent.cancel();
            if was_running {
                self.semaphore.add_permits(1);
            }
            drop(agents);
            let notifiers = self.completion_notifiers.read().await;
            if let Some(notify) = notifiers.get(id) {
                notify.notify_waiters();
            }
            Ok(())
        } else {
            Err(anyhow::anyhow!("SubAgent {} not found", id))
        }
    }

    pub async fn list(&self, status_filter: Option<SubAgentStatus>) -> Vec<SubAgent> {
        let agents = self.agents.read().await;
        match status_filter {
            Some(status) => agents
                .values()
                .filter(|a| a.status == status)
                .cloned()
                .collect(),
            None => agents.values().cloned().collect(),
        }
    }

    pub async fn list_with_content_snapshot(&self) -> Vec<SubAgent> {
        let agents = self.agents.read().await;
        agents.values().cloned().collect()
    }

    pub async fn get_agent(&self, id: &str) -> Option<SubAgent> {
        let agents = self.agents.read().await;
        agents.get(id).cloned()
    }

    pub async fn send_input(&self, id: &str, _input: String) -> Result<()> {
        let agents = self.agents.read().await;
        if agents.contains_key(id) {
            Ok(())
        } else {
            Err(anyhow::anyhow!("SubAgent {} not found", id))
        }
    }

    pub async fn resume(&self, id: &str) -> Result<()> {
        let mut agents = self.agents.write().await;
        if let Some(agent) = agents.get_mut(id) {
            agent.resume();
            Ok(())
        } else {
            Err(anyhow::anyhow!("SubAgent {} not found", id))
        }
    }

    pub async fn shutdown(&self) {
        let mut agents = self.agents.write().await;
        for agent in agents.values_mut() {
            if agent.status == SubAgentStatus::Running {
                agent.cancel();
            }
        }
        tracing::info!("SubAgentManager shutdown complete");
    }

    pub async fn get_running_count(&self) -> usize {
        let agents = self.agents.read().await;
        agents
            .values()
            .filter(|a| a.status == SubAgentStatus::Running)
            .count()
    }

    pub async fn try_acquire_slot(&self) -> bool {
        self.semaphore.try_acquire().is_ok()
    }
}

impl Default for SubAgentManager {
    fn default() -> Self {
        Self::new()
    }
}
