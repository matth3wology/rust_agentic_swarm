use anyhow::{Context, Result, anyhow};
use async_trait::async_trait;
use reqwest::header::{CONTENT_TYPE, HeaderMap, HeaderValue};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::memory::Message;

const ANTHROPIC_URL: &str = "https://api.anthropic.com/v1/messages";
const OPENAI_URL: &str = "https://api.openai.com/v1/chat/completions";
const DEFAULT_ANTHROPIC_MODEL: &str = "claude-sonnet-4-20250514";
const DEFAULT_OPENAI_MODEL: &str = "gpt-4o-mini";

/// LLM step result for the agent loop.
#[derive(Debug, Clone, PartialEq)]
pub enum Response {
    /// Requests execution of a tool with JSON args.
    ToolCall { name: String, args: Value },
    /// Final text response to return to the user.
    Final(String),
}

/// Behavior required by any model client used by the agent.
#[async_trait]
pub trait LlmClient: Send + Sync {
    /// Completes a turn with optional tool definitions.
    async fn complete_with_tools(&self, messages: &[Message], tools: &[Value]) -> Result<Response>;
}

/// Runtime-selectable provider wrapper used by the app.
pub enum AnyLlmClient {
    Anthropic(AnthropicClient),
    OpenAi(OpenAiClient),
}

#[async_trait]
impl LlmClient for AnyLlmClient {
    async fn complete_with_tools(&self, messages: &[Message], tools: &[Value]) -> Result<Response> {
        match self {
            Self::Anthropic(client) => client.complete_with_tools(messages, tools).await,
            Self::OpenAi(client) => client.complete_with_tools(messages, tools).await,
        }
    }
}

/// Creates an LLM client for a given swarm role (or `supervisor`) using env overrides.
pub fn client_for_role(role: &str) -> Result<AnyLlmClient> {
    let (provider, model_override) = provider_and_model_for_role(role);
    build_client(&provider, model_override)
}

fn build_client(provider: &str, model_override: Option<String>) -> Result<AnyLlmClient> {
    match provider.to_lowercase().as_str() {
        "anthropic" => Ok(AnyLlmClient::Anthropic(AnthropicClient::new(model_override)?)),
        "openai" => Ok(AnyLlmClient::OpenAi(OpenAiClient::new(model_override)?)),
        other => Err(anyhow!(
            "unsupported LLM_PROVIDER `{other}`; expected `anthropic` or `openai`"
        )),
    }
}

fn provider_and_model_for_role(role: &str) -> (String, Option<String>) {
    let global_provider = std::env::var("LLM_PROVIDER").unwrap_or_else(|_| "anthropic".to_string());
    let normalized = normalize_role_key(role);

    if normalized == "SUPERVISOR" {
        let provider = std::env::var("SUPERVISOR_LLM_PROVIDER").unwrap_or(global_provider);
        let model = std::env::var("SUPERVISOR_MODEL").ok();
        return (provider, model);
    }

    let role_provider_key = format!("WORKER_{normalized}_LLM_PROVIDER");
    let role_model_key = format!("WORKER_{normalized}_MODEL");
    let provider = std::env::var(role_provider_key).unwrap_or(global_provider);
    let model = std::env::var(role_model_key).ok();
    (provider, model)
}

fn normalize_role_key(role: &str) -> String {
    let mut out = String::with_capacity(role.len());
    for ch in role.chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch.to_ascii_uppercase());
        } else {
            out.push('_');
        }
    }
    out
}

/// Anthropic Messages API client.
pub struct AnthropicClient {
    client: reqwest::Client,
    api_key: String,
    model: String,
}

impl AnthropicClient {
    /// Creates a client using `ANTHROPIC_API_KEY` from environment.
    pub fn new(model_override: Option<String>) -> Result<Self> {
        let api_key = std::env::var("ANTHROPIC_API_KEY")
            .context("ANTHROPIC_API_KEY is not set; add it to .env")?;
        Ok(Self {
            client: reqwest::Client::new(),
            api_key,
            model: model_override
                .or_else(|| std::env::var("ANTHROPIC_MODEL").ok())
                .unwrap_or_else(|| DEFAULT_ANTHROPIC_MODEL.to_string()),
        })
    }

    fn headers(&self) -> Result<HeaderMap> {
        let mut headers = HeaderMap::new();
        headers.insert(
            "x-api-key",
            HeaderValue::from_str(&self.api_key).context("invalid API key header")?,
        );
        headers.insert("anthropic-version", HeaderValue::from_static("2023-06-01"));
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
        Ok(headers)
    }
}

#[async_trait]
impl LlmClient for AnthropicClient {
    async fn complete_with_tools(&self, messages: &[Message], tools: &[Value]) -> Result<Response> {
        let (system, payload_messages) = build_payload_messages(messages)?;
        let payload = json!({
            "model": self.model,
            "max_tokens": 1024,
            "system": system,
            "messages": payload_messages,
            "tools": tools
        });

        let res = self
            .client
            .post(ANTHROPIC_URL)
            .headers(self.headers()?)
            .json(&payload)
            .send()
            .await?
            .error_for_status()?;

        let body: AnthropicResponse = res.json().await?;
        parse_response(body)
    }
}

/// OpenAI Chat Completions API client.
pub struct OpenAiClient {
    client: reqwest::Client,
    api_key: String,
    model: String,
}

impl OpenAiClient {
    /// Creates a client using `OPENAI_API_KEY` from environment.
    pub fn new(model_override: Option<String>) -> Result<Self> {
        let api_key = std::env::var("OPENAI_API_KEY")
            .context("OPENAI_API_KEY is not set; add it to .env")?;
        Ok(Self {
            client: reqwest::Client::new(),
            api_key,
            model: model_override
                .or_else(|| std::env::var("OPENAI_MODEL").ok())
                .unwrap_or_else(|| DEFAULT_OPENAI_MODEL.to_string()),
        })
    }
}

#[async_trait]
impl LlmClient for OpenAiClient {
    async fn complete_with_tools(&self, messages: &[Message], tools: &[Value]) -> Result<Response> {
        let payload_messages = build_openai_messages(messages)?;
        let openai_tools = build_openai_tools(tools);
        let payload = json!({
            "model": self.model,
            "messages": payload_messages,
            "tools": openai_tools,
            "tool_choice": "auto"
        });

        let res = self
            .client
            .post(OPENAI_URL)
            .bearer_auth(&self.api_key)
            .header(CONTENT_TYPE, "application/json")
            .json(&payload)
            .send()
            .await?
            .error_for_status()?;

        let body: OpenAiResponse = res.json().await?;
        parse_openai_response(body)
    }
}

fn build_payload_messages(messages: &[Message]) -> Result<(String, Vec<Value>)> {
    let mut system = String::new();
    let mut out = Vec::new();

    for (idx, msg) in messages.iter().enumerate() {
        match msg.role.as_str() {
            "system" => {
                system = msg.content.clone();
            }
            "user" | "assistant" => {
                out.push(json!({
                    "role": msg.role,
                    "content": [{
                        "type": "text",
                        "text": msg.content
                    }]
                }));
            }
            "tool" => {
                let tool_name = msg
                    .tool_name
                    .clone()
                    .ok_or_else(|| anyhow!("tool message missing tool_name"))?;
                out.push(json!({
                    "role": "user",
                    "content": [{
                        "type": "tool_result",
                        "tool_use_id": format!("tool-{idx}"),
                        "content": msg.content
                    },{
                        "type": "text",
                        "text": format!("Result is from tool `{tool_name}`.")
                    }]
                }));
            }
            other => return Err(anyhow!("unsupported role `{other}`")),
        }
    }
    Ok((system, out))
}

fn parse_response(body: AnthropicResponse) -> Result<Response> {
    for block in &body.content {
        if block.r#type == "tool_use" {
            let name = block.name.clone().ok_or_else(|| anyhow!("missing tool name"))?;
            let args = block.input.clone().unwrap_or(Value::Null);
            return Ok(Response::ToolCall { name, args });
        }
    }

    let text = body
        .content
        .iter()
        .filter(|b| b.r#type == "text")
        .filter_map(|b| b.text.clone())
        .collect::<Vec<_>>()
        .join("\n");

    if text.trim().is_empty() {
        return Err(anyhow!("model returned neither tool_use nor text"));
    }
    Ok(Response::Final(text))
}

fn build_openai_tools(tools: &[Value]) -> Vec<Value> {
    tools
        .iter()
        .map(|tool| {
            json!({
                "type": "function",
                "function": {
                    "name": tool.get("name").and_then(Value::as_str).unwrap_or("tool"),
                    "description": tool.get("description").and_then(Value::as_str).unwrap_or(""),
                    "parameters": tool.get("input_schema").cloned().unwrap_or_else(|| json!({"type":"object"}))
                }
            })
        })
        .collect()
}

fn build_openai_messages(messages: &[Message]) -> Result<Vec<Value>> {
    let mut out = Vec::new();
    for msg in messages {
        match msg.role.as_str() {
            "system" | "user" | "assistant" => {
                out.push(json!({
                    "role": msg.role,
                    "content": msg.content
                }));
            }
            "tool" => {
                let tool_name = msg
                    .tool_name
                    .clone()
                    .ok_or_else(|| anyhow!("tool message missing tool_name"))?;
                out.push(json!({
                    "role": "tool",
                    "name": tool_name,
                    "content": msg.content
                }));
            }
            other => return Err(anyhow!("unsupported role `{other}`")),
        }
    }
    Ok(out)
}

fn parse_openai_response(body: OpenAiResponse) -> Result<Response> {
    let choice = body
        .choices
        .into_iter()
        .next()
        .ok_or_else(|| anyhow!("openai returned no choices"))?;

    if let Some(tool_calls) = choice.message.tool_calls {
        for call in tool_calls {
            if call.r#type == "function" {
                let args = serde_json::from_str::<Value>(&call.function.arguments)
                    .unwrap_or(Value::Null);
                return Ok(Response::ToolCall {
                    name: call.function.name,
                    args,
                });
            }
        }
    }

    let text = choice.message.content.unwrap_or_default();
    if text.trim().is_empty() {
        return Err(anyhow!("openai returned neither tool_calls nor text"));
    }
    Ok(Response::Final(text))
}

#[derive(Debug, Deserialize)]
struct AnthropicResponse {
    content: Vec<ContentBlock>,
}

#[derive(Debug, Deserialize, Serialize)]
struct ContentBlock {
    #[serde(rename = "type")]
    r#type: String,
    text: Option<String>,
    name: Option<String>,
    input: Option<Value>,
}

#[derive(Debug, Deserialize)]
struct OpenAiResponse {
    choices: Vec<OpenAiChoice>,
}

#[derive(Debug, Deserialize)]
struct OpenAiChoice {
    message: OpenAiMessage,
}

#[derive(Debug, Deserialize)]
struct OpenAiMessage {
    content: Option<String>,
    tool_calls: Option<Vec<OpenAiToolCall>>,
}

#[derive(Debug, Deserialize)]
struct OpenAiToolCall {
    #[serde(rename = "type")]
    r#type: String,
    function: OpenAiFunctionCall,
}

#[derive(Debug, Deserialize)]
struct OpenAiFunctionCall {
    name: String,
    arguments: String,
}

#[cfg(test)]
mod tests {
    use super::{
        AnthropicResponse, ContentBlock, OpenAiChoice, OpenAiFunctionCall, OpenAiMessage,
        OpenAiResponse, OpenAiToolCall, Response, normalize_role_key, parse_openai_response,
        parse_response,
    };

    #[test]
    fn parses_tool_call_response() {
        let body = AnthropicResponse {
            content: vec![ContentBlock {
                r#type: "tool_use".to_string(),
                text: None,
                name: Some("web_search".to_string()),
                input: Some(serde_json::json!({"query":"rust"})),
            }],
        };

        let parsed = parse_response(body).expect("should parse");
        assert!(matches!(parsed, Response::ToolCall { .. }));
    }

    #[test]
    fn parses_final_text_response() {
        let body = AnthropicResponse {
            content: vec![ContentBlock {
                r#type: "text".to_string(),
                text: Some("done".to_string()),
                name: None,
                input: None,
            }],
        };
        let parsed = parse_response(body).expect("should parse");
        assert_eq!(parsed, Response::Final("done".to_string()));
    }

    #[test]
    fn parses_openai_tool_call_response() {
        let body = OpenAiResponse {
            choices: vec![OpenAiChoice {
                message: OpenAiMessage {
                    content: None,
                    tool_calls: Some(vec![OpenAiToolCall {
                        r#type: "function".to_string(),
                        function: OpenAiFunctionCall {
                            name: "web_search".to_string(),
                            arguments: "{\"query\":\"rust\"}".to_string(),
                        },
                    }]),
                },
            }],
        };
        let parsed = parse_openai_response(body).expect("should parse");
        assert!(matches!(parsed, Response::ToolCall { .. }));
    }

    #[test]
    fn parses_openai_text_response() {
        let body = OpenAiResponse {
            choices: vec![OpenAiChoice {
                message: OpenAiMessage {
                    content: Some("done".to_string()),
                    tool_calls: None,
                },
            }],
        };
        let parsed = parse_openai_response(body).expect("should parse");
        assert_eq!(parsed, Response::Final("done".to_string()));
    }

    #[test]
    fn normalizes_role_keys_for_env_lookup() {
        assert_eq!(normalize_role_key("researcher"), "RESEARCHER");
        assert_eq!(normalize_role_key("fact-checker"), "FACT_CHECKER");
    }
}
