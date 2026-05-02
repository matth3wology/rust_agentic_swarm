use crate::memory::Message;

/// ReAct-style planner helpers for prompt shaping.
pub struct ReActPlanner;

impl ReActPlanner {
    /// Builds an augmented system prompt that enforces tool-aware reasoning.
    pub fn system_prompt(base: &str) -> String {
        format!(
            "{base}\nUse ReAct: think step-by-step, call tools when needed, then provide a final answer."
        )
    }

    /// Produces a lightweight reflection string from a tool result.
    pub fn reflect(tool_name: &str, tool_output: &str) -> String {
        format!("Tool `{tool_name}` returned: {tool_output}")
    }

    /// Returns true when there are recent tool messages to reason over.
    pub fn has_recent_tool_signal(messages: &[Message]) -> bool {
        messages.iter().rev().take(3).any(|m| m.role == "tool")
    }
}

#[cfg(test)]
mod tests {
    use super::ReActPlanner;
    use crate::memory::Message;

    #[test]
    fn detects_recent_tool_messages() {
        let messages = vec![Message {
            role: "tool".to_string(),
            content: "ok".to_string(),
            tool_name: Some("file_io".to_string()),
        }];
        assert!(ReActPlanner::has_recent_tool_signal(&messages));
    }
}
