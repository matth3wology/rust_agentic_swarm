use std::collections::{HashMap, VecDeque};

/// A single conversational message used for agent context.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct Message {
    /// The role of the message sender: user, assistant, or tool.
    pub role: String,
    /// Human-readable content for the model.
    pub content: String,
    /// Optional tool name when role is `tool`.
    pub tool_name: Option<String>,
}

/// A memory store abstraction for long-term retrieval.
pub trait MemoryStore: Send + Sync {
    /// Saves a value under a key.
    fn save(&mut self, key: String, value: String);
    /// Returns the top-k matches for a query.
    fn search(&self, query: &str, top_k: usize) -> Vec<(String, String)>;
}

/// In-memory long-term memory implementation.
#[derive(Debug, Default)]
pub struct InMemoryStore {
    entries: HashMap<String, String>,
}

impl MemoryStore for InMemoryStore {
    fn save(&mut self, key: String, value: String) {
        self.entries.insert(key, value);
    }

    fn search(&self, query: &str, top_k: usize) -> Vec<(String, String)> {
        let mut hits = self
            .entries
            .iter()
            .filter(|(k, v)| k.contains(query) || v.contains(query))
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect::<Vec<_>>();
        hits.truncate(top_k);
        hits
    }
}

/// Combined short-term and long-term memory manager.
pub struct Memory {
    short_term: VecDeque<Message>,
    max_messages: usize,
    long_term: Box<dyn MemoryStore>,
}

impl Memory {
    /// Creates memory with bounded short-term history.
    pub fn new(max_messages: usize) -> Self {
        Self {
            short_term: VecDeque::new(),
            max_messages,
            long_term: Box::<InMemoryStore>::default(),
        }
    }

    /// Adds a user message to short-term memory.
    pub fn add_user(&mut self, content: impl Into<String>) {
        self.push(Message {
            role: "user".to_string(),
            content: content.into(),
            tool_name: None,
        });
    }

    /// Adds an assistant message to short-term memory.
    pub fn add_assistant(&mut self, content: impl Into<String>) {
        self.push(Message {
            role: "assistant".to_string(),
            content: content.into(),
            tool_name: None,
        });
    }

    /// Adds a tool result message to short-term memory.
    pub fn add_tool_result(&mut self, tool_name: impl Into<String>, content: impl Into<String>) {
        self.push(Message {
            role: "tool".to_string(),
            content: content.into(),
            tool_name: Some(tool_name.into()),
        });
    }

    /// Builds model context prefixed with a system message.
    pub fn build_context(&self, system_prompt: &str) -> Vec<Message> {
        let mut ctx = vec![Message {
            role: "system".to_string(),
            content: system_prompt.to_string(),
            tool_name: None,
        }];
        ctx.extend(self.short_term.iter().cloned());
        ctx
    }

    /// Saves data into long-term memory.
    pub fn save_long_term(&mut self, key: String, value: String) {
        self.long_term.save(key, value);
    }

    /// Searches long-term memory.
    pub fn search_long_term(&self, query: &str, top_k: usize) -> Vec<(String, String)> {
        self.long_term.search(query, top_k)
    }

    fn push(&mut self, msg: Message) {
        self.short_term.push_back(msg);
        while self.short_term.len() > self.max_messages {
            let _ = self.short_term.pop_front();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Memory;

    #[test]
    fn short_term_memory_is_bounded() {
        let mut memory = Memory::new(2);
        memory.add_user("one");
        memory.add_assistant("two");
        memory.add_user("three");

        let ctx = memory.build_context("sys");
        assert_eq!(ctx.len(), 3);
        assert_eq!(ctx[1].content, "two");
        assert_eq!(ctx[2].content, "three");
    }

    #[test]
    fn long_term_memory_searches() {
        let mut memory = Memory::new(3);
        memory.save_long_term("rust".to_string(), "great language".to_string());
        let results = memory.search_long_term("rust", 1);
        assert_eq!(results.len(), 1);
    }
}
