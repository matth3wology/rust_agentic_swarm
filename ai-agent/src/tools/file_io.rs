use anyhow::{Result, anyhow};
use async_trait::async_trait;
use serde_json::{Value, json};
use tokio::fs;

use super::Tool;

/// File read/write tool for local workspace operations.
pub struct FileIoTool;

impl FileIoTool {
    /// Creates a new file I/O tool.
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl Tool for FileIoTool {
    fn name(&self) -> &str {
        "file_io"
    }

    fn description(&self) -> &str {
        "Reads or writes a file on disk."
    }

    fn schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "action": { "type": "string", "enum": ["read", "write"] },
                "path": { "type": "string" },
                "content": { "type": "string" }
            },
            "required": ["action", "path"],
            "additionalProperties": false
        })
    }

    async fn call(&self, args: Value) -> Result<String> {
        let action = args
            .get("action")
            .and_then(Value::as_str)
            .ok_or_else(|| anyhow!("missing `action`"))?;
        let path = args
            .get("path")
            .and_then(Value::as_str)
            .ok_or_else(|| anyhow!("missing `path`"))?;

        match action {
            "read" => {
                let content = fs::read_to_string(path).await?;
                Ok(content)
            }
            "write" => {
                let content = args
                    .get("content")
                    .and_then(Value::as_str)
                    .ok_or_else(|| anyhow!("missing `content` for write"))?;
                fs::write(path, content).await?;
                Ok(format!("wrote {} bytes to {path}", content.len()))
            }
            _ => Err(anyhow!("unsupported action: {action}")),
        }
    }
}

#[cfg(test)]
mod tests {
    use anyhow::Result;
    use serde_json::json;
    use tokio::fs;

    use super::FileIoTool;
    use crate::tools::Tool;

    #[tokio::test]
    async fn file_io_writes_and_reads() -> Result<()> {
        let tool = FileIoTool::new();
        let path = std::env::temp_dir().join("agent_file_io_test.txt");
        let path_str = path.to_string_lossy().to_string();

        let _ = tool
            .call(json!({
                "action": "write",
                "path": path_str,
                "content": "hello"
            }))
            .await?;
        let read = tool
            .call(json!({
                "action": "read",
                "path": path.to_string_lossy().to_string()
            }))
            .await?;

        assert_eq!(read, "hello");
        let _ = fs::remove_file(path).await;
        Ok(())
    }
}
