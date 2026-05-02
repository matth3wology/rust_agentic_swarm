use std::collections::HashMap;
use std::sync::Arc;

use anyhow::{Result, anyhow};
use async_trait::async_trait;
use serde_json::Value;

mod file_io;
mod web_search;

pub use file_io::FileIoTool;
pub use web_search::WebSearchTool;

/// A callable tool exposed to the LLM.
#[async_trait]
pub trait Tool: Send + Sync {
    /// Unique tool name used for dispatch.
    fn name(&self) -> &str;
    /// Human-readable tool description for model planning.
    fn description(&self) -> &str;
    /// JSON schema describing valid input arguments.
    fn schema(&self) -> Value;
    /// Executes the tool with JSON arguments and returns string output.
    async fn call(&self, args: Value) -> Result<String>;
}

/// Runtime registry for all available tools.
#[derive(Clone)]
pub struct ToolRegistry {
    tools: HashMap<String, Arc<dyn Tool>>,
}

impl ToolRegistry {
    /// Creates an empty tool registry.
    pub fn new() -> Self {
        Self {
            tools: HashMap::new(),
        }
    }

    /// Registers a tool instance by its name.
    pub fn register<T>(&mut self, tool: T)
    where
        T: Tool + 'static,
    {
        self.tools.insert(tool.name().to_string(), Arc::new(tool));
    }

    /// Calls a tool by name with JSON arguments.
    pub async fn call(&self, name: &str, args: Value) -> Result<String> {
        let tool = self
            .tools
            .get(name)
            .ok_or_else(|| anyhow!("tool not found: {name}"))?;
        tool.call(args).await
    }

    /// Exposes tool schemas in Anthropic-compatible format.
    pub fn schemas(&self) -> Vec<Value> {
        self.tools
            .values()
            .map(|tool| {
                serde_json::json!({
                    "name": tool.name(),
                    "description": tool.description(),
                    "input_schema": tool.schema(),
                })
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use anyhow::Result;
    use serde_json::json;

    use super::{ToolRegistry, WebSearchTool};

    #[tokio::test]
    async fn registry_dispatches_tools() -> Result<()> {
        let mut registry = ToolRegistry::new();
        registry.register(WebSearchTool::new());

        let output = registry
            .call("web_search", json!({"query": "rust async trait"}))
            .await?;
        assert!(output.contains("Result"));
        Ok(())
    }
}
