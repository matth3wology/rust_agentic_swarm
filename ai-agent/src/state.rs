/// Represents the current lifecycle state of the agent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AgentState {
    /// Waiting for new user input.
    Idle,
    /// Reading user/tool observations.
    Perceiving,
    /// Producing a reasoning step with the model.
    Reasoning,
    /// Dispatching a named tool.
    UsingTool { tool_name: String },
    /// Reflecting over tool results and next action.
    Reflecting,
    /// Returning a final answer to the user.
    Responding,
    /// Terminal error state with explanation.
    Error(String),
}

#[cfg(test)]
mod tests {
    use super::AgentState;

    #[test]
    fn using_tool_state_carries_name() {
        let state = AgentState::UsingTool {
            tool_name: "web_search".to_string(),
        };
        match state {
            AgentState::UsingTool { tool_name } => assert_eq!(tool_name, "web_search"),
            _ => panic!("unexpected state variant"),
        }
    }
}
