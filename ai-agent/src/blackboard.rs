use std::fmt::Write as _;

/// A single contribution from a worker agent to the shared blackboard.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct BlackboardEntry {
    /// Worker identifier, usually role + index.
    pub worker_id: String,
    /// Worker role or specialization.
    pub role: String,
    /// Worker-produced claim/summary.
    pub content: String,
    /// Optional confidence score in [0.0, 1.0].
    pub confidence: Option<f32>,
    /// Optional error details if the worker failed.
    pub error: Option<String>,
}

/// Shared blackboard state consumed by the supervisor.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct Blackboard {
    entries: Vec<BlackboardEntry>,
}

impl Blackboard {
    /// Creates an empty blackboard.
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    /// Appends an entry to the board.
    pub fn add(&mut self, entry: BlackboardEntry) {
        self.entries.push(entry);
    }

    /// Returns all entries.
    pub fn entries(&self) -> &[BlackboardEntry] {
        &self.entries
    }

    /// Renders entries in a compact prompt-ready format.
    pub fn render_for_prompt(&self) -> String {
        let mut out = String::new();
        for (idx, entry) in self.entries.iter().enumerate() {
            let _ = writeln!(out, "Entry {}:", idx + 1);
            let _ = writeln!(out, "- worker_id: {}", entry.worker_id);
            let _ = writeln!(out, "- role: {}", entry.role);
            if let Some(confidence) = entry.confidence {
                let _ = writeln!(out, "- confidence: {:.2}", confidence);
            }
            if let Some(err) = &entry.error {
                let _ = writeln!(out, "- error: {err}");
            }
            let _ = writeln!(out, "- content:\n{}", entry.content);
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::{Blackboard, BlackboardEntry};

    #[test]
    fn blackboard_renders_entries() {
        let mut board = Blackboard::new();
        board.add(BlackboardEntry {
            worker_id: "worker-1".to_string(),
            role: "researcher".to_string(),
            content: "Found 3 relevant facts.".to_string(),
            confidence: Some(0.8),
            error: None,
        });
        let rendered = board.render_for_prompt();
        assert!(rendered.contains("worker-1"));
        assert!(rendered.contains("researcher"));
    }
}
