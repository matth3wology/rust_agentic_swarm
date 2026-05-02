use anyhow::Result;

use crate::agent::Agent;
use crate::blackboard::Blackboard;
use crate::llm::LlmClient;
use crate::memory::Memory;
use crate::tools::ToolRegistry;

/// Supervisor that synthesizes a final decision from blackboard entries.
pub struct Supervisor<L: LlmClient> {
    llm: L,
    max_steps: usize,
}

impl<L: LlmClient> Supervisor<L> {
    /// Creates a supervisor with a model client and step budget.
    pub fn new(llm: L, max_steps: usize) -> Self {
        Self { llm, max_steps }
    }

    /// Produces a final decision from user request + blackboard content.
    pub async fn decide(self, user_input: &str, blackboard: &Blackboard) -> Result<String> {
        let system_prompt = "You are the supervisor agent. Consolidate worker blackboard entries into a final decision. Resolve conflicts explicitly and state confidence."
            .to_string();
        let mut memory = Memory::new(40);
        memory.add_user(format!("Original user request:\n{user_input}"));
        memory.add_user(format!(
            "Shared blackboard entries:\n{}",
            blackboard.render_for_prompt()
        ));
        memory.add_user("Return one final decision with short rationale and any remaining uncertainty.");

        let tools = ToolRegistry::new();
        let mut agent = Agent::new(self.llm, tools, memory, system_prompt, self.max_steps);
        agent.run("Produce final supervisor decision").await
    }
}
