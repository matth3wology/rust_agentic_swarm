use anyhow::{Result, anyhow};
use async_trait::async_trait;
use serde_json::{Value, json};

use super::Tool;

/// Simple web search tool with mock output.
pub struct WebSearchTool;

impl WebSearchTool {
    /// Creates a new web search tool.
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl Tool for WebSearchTool {
    fn name(&self) -> &str {
        "web_search"
    }

    fn description(&self) -> &str {
        "Searches the web for relevant information."
    }

    fn schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "query": { "type": "string", "description": "Search query" }
            },
            "required": ["query"],
            "additionalProperties": false
        })
    }

    async fn call(&self, args: Value) -> Result<String> {
        let query = args
            .get("query")
            .and_then(Value::as_str)
            .ok_or_else(|| anyhow!("missing `query`"))?;
        Ok(format!(
            "Result 1 for '{query}': https://example.com/a\nResult 2 for '{query}': https://example.com/b"
        ))
    }
}

#[cfg(test)]
mod tests {
    use anyhow::Result;
    use serde_json::json;

    use super::WebSearchTool;
    use crate::tools::Tool;

    #[tokio::test]
    async fn web_search_returns_mock_results() -> Result<()> {
        let tool = WebSearchTool::new();
        let out = tool.call(json!({"query": "Rust"})).await?;
        assert!(out.contains("Result 1"));
        Ok(())
    }
}
