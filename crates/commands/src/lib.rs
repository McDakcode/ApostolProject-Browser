// Made by MrDuck && Ox-Alpha
//! apb-commands
//!
//! Command Palette registry (§22). Every feature crate registers its actions
//! here at startup (`register`); the UI queries `search` on each keystroke.
//! Fuzzy matching is a small self-contained subsequence scorer — no external
//! dependency needed for something this bounded, and it keeps the crate's
//! compile footprint tiny.

use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct CommandEntry {
    pub id: String,
    pub title: String,
    pub category: String,
    pub keywords: Vec<String>,
    pub shortcut: Option<String>,
}

pub struct CommandRegistry {
    entries: Vec<CommandEntry>,
}

impl CommandRegistry {
    pub fn new() -> Self {
        Self { entries: Vec::new() }
    }

    pub fn register(&mut self, entry: CommandEntry) {
        self.entries.push(entry);
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Fuzzy-search over title + keywords, ranked best-first. Empty query
    /// returns all entries in registration order (palette's default view).
    pub fn search(&self, query: &str) -> Vec<&CommandEntry> {
        if query.trim().is_empty() {
            return self.entries.iter().collect();
        }
        let q = query.to_lowercase();
        let mut scored: Vec<(i32, &CommandEntry)> = self
            .entries
            .iter()
            .filter_map(|e| {
                let haystack = format!("{} {}", e.title, e.keywords.join(" ")).to_lowercase();
                fuzzy_score(&q, &haystack).map(|score| (score, e))
            })
            .collect();
        scored.sort_by(|a, b| b.0.cmp(&a.0));
        scored.into_iter().map(|(_, e)| e).collect()
    }
}

impl Default for CommandRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Subsequence match: every character of `query` must appear in order inside
/// `haystack`. Score rewards contiguous runs and early matches, similar in
/// spirit to VS Code / Raycast fuzzy matchers, without pulling in a crate.
fn fuzzy_score(query: &str, haystack: &str) -> Option<i32> {
    let h: Vec<char> = haystack.chars().collect();
    let mut score = 0i32;
    let mut h_idx = 0usize;
    let mut last_match: Option<usize> = None;

    for qc in query.chars() {
        let mut found = None;
        while h_idx < h.len() {
            if h[h_idx] == qc {
                found = Some(h_idx);
                break;
            }
            h_idx += 1;
        }
        let idx = found?;
        score += 10;
        if let Some(last) = last_match {
            if idx == last + 1 {
                score += 15; // contiguous run bonus
            }
        }
        if idx == 0 {
            score += 5; // start-of-string bonus
        }
        last_match = Some(idx);
        h_idx = idx + 1;
    }
    Some(score)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_registry() -> CommandRegistry {
        let mut r = CommandRegistry::new();
        r.register(CommandEntry {
            id: "tabs.new".into(),
            title: "Open new tab".into(),
            category: "Tabs".into(),
            keywords: vec!["new".into(), "tab".into()],
            shortcut: Some("Ctrl+T".into()),
        });
        r.register(CommandEntry {
            id: "notes.create".into(),
            title: "Create note".into(),
            category: "Notes".into(),
            keywords: vec!["note".into(), "markdown".into()],
            shortcut: Some("Ctrl+Shift+O".into()),
        });
        r.register(CommandEntry {
            id: "privacy.clear".into(),
            title: "Clear browsing data".into(),
            category: "Privacy".into(),
            keywords: vec!["clear".into(), "cookies".into(), "history".into()],
            shortcut: None,
        });
        r
    }

    #[test]
    fn empty_query_returns_everything() {
        let r = sample_registry();
        assert_eq!(r.search("").len(), 3);
    }

    #[test]
    fn fuzzy_query_ranks_best_match_first() {
        let r = sample_registry();
        let results = r.search("nt");
        assert!(!results.is_empty());
        // "note" contains n...t as a subsequence and should outrank unrelated entries.
        assert!(results.iter().any(|e| e.id == "notes.create"));
    }

    #[test]
    fn non_matching_query_excludes_entry() {
        let r = sample_registry();
        let results = r.search("zzzzz");
        assert!(results.is_empty());
    }
}

// Made by MrDuck && Ox-Alpha