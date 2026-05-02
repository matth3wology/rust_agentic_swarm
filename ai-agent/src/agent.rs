use anyhow::{Result, anyhow};
use tracing::info;

use crate::llm::{LlmClient, Response};
use crate::memory::Memory;
use crate::planner::ReActPlanner;
use crate::state::AgentState;
use crate::tools::ToolRegistry;

/// Main ReAct agent orchestration loop.
pub struct Agent<L: LlmClient> {
    /// Model client.
    pub llm: L,
    /// Tool runtime registry.
    pub tools: ToolRegistry,
    /// Memory manager.
    pub memory: Memory,
    /// Base system prompt.
    pub system_prompt: String,
    /// Maximum reasoning steps before hard stop.
    pub max_steps: usize,
    /// Current state of the agent.
    pub state: AgentState,
}

impl<L: LlmClient> Agent<L> {
    /// Creates a new agent instance.
    pub fn new(
        llm: L,
        tools: ToolRegistry,
        memory: Memory,
        system_prompt: String,
        max_steps: usize,
    ) -> Self {
        Self {
            llm,
            tools,
            memory,
            system_prompt,
            max_steps,
            state: AgentState::Idle,
        }
    }

    /// Runs the agent loop for a single user input.
    #[tracing::instrument(skip(self, user_input), fields(max_steps = self.max_steps))]
    pub async fn run(&mut self, user_input: &str) -> Result<String> {
        self.state = AgentState::Perceiving;
        self.memory.add_user(user_input.to_string());
        info!("state=Perceiving user_input_received");

        for step in 1..=self.max_steps {
            self.state = AgentState::Reasoning;
            info!("state=Reasoning step={step}");

            let prompt = ReActPlanner::system_prompt(&self.system_prompt);
            let context = self.memory.build_context(&prompt);
            let response = self
                .llm
                .complete_with_tools(&context, &self.tools.schemas())
                .await?;

            match response {
                Response::ToolCall { name, args } => {
                    self.state = AgentState::UsingTool {
                        tool_name: name.clone(),
                    };
                    info!("state=UsingTool step={step} tool={name}");
                    let tool_output = self.tools.call(&name, args).await?;
                    self.memory.add_tool_result(name.clone(), tool_output.clone());

                    self.state = AgentState::Reflecting;
                    let reflection = ReActPlanner::reflect(&name, &tool_output);
                    info!("state=Reflecting step={step} reflection={reflection}");
                }
                Response::Final(answer) => {
                    self.state = AgentState::Responding;
                    info!("state=Responding step={step}");
                    self.memory.add_assistant(answer.clone());
                    return Ok(answer);
                }
            }
        }

        self.state = AgentState::Error("max steps reached".to_string());
        Err(anyhow!("max steps reached before final response"))
    }
}

#[cfg(test)]
mod tests {
    use anyhow::Result;
    use async_trait::async_trait;
    use serde_json::Value;
    use std::sync::Mutex;

    use crate::llm::{LlmClient, Response};
    use crate::memory::{Memory, Message};
    use crate::tools::{ToolRegistry, WebSearchTool};

    use super::Agent;

    struct FakeLlm {
        sequence: Mutex<Vec<Response>>,
    }

    #[async_trait]
    impl LlmClient for FakeLlm {
        async fn complete_with_tools(
            &self,
            _messages: &[Message],
            _tools: &[Value],
        ) -> Result<Response> {
            let mut guard = self.sequence.lock().map_err(|_| anyhow::anyhow!("poisoned"))?;
            if guard.is_empty() {
                return Ok(Response::Final("done".to_string()));
            }
            Ok(guard.remove(0))
        }
    }

    #[tokio::test]
    async fn agent_runs_react_loop() -> Result<()> {
        let llm = FakeLlm {
            sequence: Mutex::new(vec![
                Response::ToolCall {
                    name: "web_search".to_string(),
                    args: serde_json::json!({"query":"rust agent"}),
                },
                Response::Final("answer ready".to_string()),
            ]),
        };
        let mut tools = ToolRegistry::new();
        tools.register(WebSearchTool::new());

        let memory = Memory::new(10);
        let mut agent = Agent::new(llm, tools, memory, "sys".to_string(), 5);
        let result = agent.run("help me").await?;
        assert_eq!(result, "answer ready");
        Ok(())
    }
}
