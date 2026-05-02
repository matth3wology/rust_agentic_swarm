use anyhow::Result;
use tokio::task::JoinSet;

use crate::agent::Agent;
use crate::blackboard::{Blackboard, BlackboardEntry};
use crate::llm::LlmClient;
use crate::memory::Memory;
use crate::supervisor::Supervisor;
use crate::tools::ToolRegistry;

/// Runtime configuration for a worker swarm + supervisor.
#[derive(Debug, Clone)]
pub struct SwarmConfig {
    pub worker_roles: Vec<String>,
    pub worker_max_steps: usize,
    pub supervisor_max_steps: usize,
    pub worker_memory_size: usize,
}

impl Default for SwarmConfig {
    fn default() -> Self {
        Self {
            worker_roles: vec![
                "researcher".to_string(),
                "critic".to_string(),
                "synthesizer".to_string(),
            ],
            worker_max_steps: 6,
            supervisor_max_steps: 6,
            worker_memory_size: 20,
        }
    }
}

/// Final output of a swarm execution.
pub struct SwarmRunResult {
    pub blackboard: Blackboard,
    pub final_decision: String,
}

/// Orchestrates worker agents that contribute to a blackboard, then runs a supervisor.
pub struct SwarmOrchestrator<F, L>
where
    F: Fn(&str) -> Result<L>,
    L: LlmClient + Send + 'static,
{
    llm_factory: F,
    tools: ToolRegistry,
    config: SwarmConfig,
}

impl<F, L> SwarmOrchestrator<F, L>
where
    F: Fn(&str) -> Result<L>,
    L: LlmClient + Send + 'static,
{
    /// Creates a new orchestrator from an LLM factory, shared tools, and config.
    pub fn new(llm_factory: F, tools: ToolRegistry, config: SwarmConfig) -> Self {
        Self {
            llm_factory,
            tools,
            config,
        }
    }

    /// Runs worker swarm and supervisor for a single user request.
    pub async fn run(&self, user_input: &str) -> Result<SwarmRunResult> {
        let mut blackboard = Blackboard::new();
        let mut worker_tasks = JoinSet::new();

        for (idx, role) in self.config.worker_roles.iter().enumerate() {
            let worker_id = format!("worker-{}-{role}", idx + 1);
            let role = role.clone();
            let worker_prompt = format!(
                "You are a specialized swarm worker with role `{role}`. Analyze the request, use tools when needed, and return structured findings for the shared blackboard."
            );
            let llm = (self.llm_factory)(&role)?;
            let tools = self.tools.clone();
            let worker_memory_size = self.config.worker_memory_size;
            let worker_max_steps = self.config.worker_max_steps;
            let user_input = user_input.to_string();

            worker_tasks.spawn(async move {
                let mut worker = Agent::new(
                    llm,
                    tools,
                    Memory::new(worker_memory_size),
                    worker_prompt,
                    worker_max_steps,
                );

                match worker.run(&user_input).await {
                    Ok(content) => BlackboardEntry {
                        worker_id,
                        role,
                        content,
                        confidence: None,
                        error: None,
                    },
                    Err(err) => BlackboardEntry {
                        worker_id,
                        role,
                        content: "Worker failed before producing valid output.".to_string(),
                        confidence: None,
                        error: Some(err.to_string()),
                    },
                }
            });
        }

        while let Some(result) = worker_tasks.join_next().await {
            match result {
                Ok(entry) => blackboard.add(entry),
                Err(err) => blackboard.add(BlackboardEntry {
                    worker_id: "worker-join-error".to_string(),
                    role: "unknown".to_string(),
                    content: "Worker task did not complete successfully.".to_string(),
                    confidence: None,
                    error: Some(err.to_string()),
                }),
            }
        }

        let supervisor_llm = (self.llm_factory)("supervisor")?;
        let supervisor = Supervisor::new(supervisor_llm, self.config.supervisor_max_steps);
        let final_decision = supervisor.decide(user_input, &blackboard).await?;

        Ok(SwarmRunResult {
            blackboard,
            final_decision,
        })
    }
}
