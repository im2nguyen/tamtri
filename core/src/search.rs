use crate::conversation::{ContentBlock, Conversation};
use crate::vault::{ConversationSummary, ConversationVault};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchHit {
    pub conversation_id: uuid::Uuid,
    pub title: String,
    pub snippet: String,
    pub match_field: SearchMatchField,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SearchMatchField {
    Title,
    Text,
    Thinking,
}

pub fn search_conversations<V: ConversationVault>(
    vault: &V,
    query: &str,
) -> crate::Result<Vec<SearchHit>> {
    let needle = query.trim();
    if needle.is_empty() {
        return Ok(Vec::new());
    }
    let needle_lower = needle.to_ascii_lowercase();
    let summaries = vault.list()?;
    let mut hits = Vec::new();
    for summary in summaries {
        if let Some(hit) = search_summary(vault, &summary, &needle_lower)? {
            hits.push(hit);
            continue;
        }
        if let Ok(conversation) = vault.load(summary.id) {
            hits.extend(search_conversation(&conversation, &needle_lower));
        }
    }
    Ok(hits)
}

fn search_summary<V: ConversationVault>(
    vault: &V,
    summary: &ConversationSummary,
    needle_lower: &str,
) -> crate::Result<Option<SearchHit>> {
    if summary.title.to_ascii_lowercase().contains(needle_lower) {
        return Ok(Some(SearchHit {
            conversation_id: summary.id,
            title: summary.title.clone(),
            snippet: summary.title.clone(),
            match_field: SearchMatchField::Title,
        }));
    }
    let _ = vault;
    Ok(None)
}

fn search_conversation(conversation: &Conversation, needle_lower: &str) -> Vec<SearchHit> {
    let mut hits = Vec::new();
    for message in &conversation.messages {
        for block in &message.content {
            match block {
                ContentBlock::Text { text } if text.to_ascii_lowercase().contains(needle_lower) => {
                    hits.push(SearchHit {
                        conversation_id: conversation.id,
                        title: conversation.title.clone(),
                        snippet: snippet_from(text, needle_lower),
                        match_field: SearchMatchField::Text,
                    });
                }
                ContentBlock::Thinking { text }
                    if text.to_ascii_lowercase().contains(needle_lower) =>
                {
                    hits.push(SearchHit {
                        conversation_id: conversation.id,
                        title: conversation.title.clone(),
                        snippet: snippet_from(text, needle_lower),
                        match_field: SearchMatchField::Thinking,
                    });
                }
                _ => {}
            }
        }
    }
    hits
}

fn snippet_from(text: &str, needle_lower: &str) -> String {
    let lower = text.to_ascii_lowercase();
    let Some(index) = lower.find(needle_lower) else {
        return text.chars().take(120).collect();
    };
    let start = index.saturating_sub(40);
    let end = (index + needle_lower.len() + 80).min(text.len());
    let slice = &text[start..end];
    if start > 0 {
        format!("…{slice}")
    } else {
        slice.to_string()
    }
}

pub const SEARCH_SCOPE_MESSAGE: &str =
    "Search covers conversation titles plus Text and Thinking blocks only.";

#[cfg(test)]
mod tests {
    use chrono::Utc;

    use crate::conversation::{ContentBlock, Conversation, Message, Role};
    use crate::vault::ConversationVault;
    use crate::vault::fs::FilesystemVault;

    use super::*;

    fn conversation_with_blocks(title: &str, blocks: Vec<ContentBlock>) -> Conversation {
        let mut conversation = Conversation::new(title);
        conversation.push_message(Message {
            id: uuid::Uuid::now_v7(),
            role: Role::Assistant,
            harness_id: None,
            content: blocks,
            created_at: Utc::now(),
        });
        conversation
    }

    #[test]
    fn search_matches_titles_text_and_thinking_only() {
        let temp = tempfile::tempdir().expect("tempdir");
        let vault = FilesystemVault::new(temp.path()).expect("vault");
        let report = conversation_with_blocks(
            "Quarterly report",
            vec![
                ContentBlock::Text {
                    text: "Revenue grew in Q1".into(),
                },
                ContentBlock::Thinking {
                    text: "Need to mention CSV source".into(),
                },
                ContentBlock::ToolResult {
                    call_id: "tool-1".into(),
                    output: serde_json::json!({"secret": "hidden keyword"}),
                },
            ],
        );
        vault.create(&report).expect("create");

        let title_hits = search_conversations(&vault, "Quarterly").expect("search");
        assert_eq!(title_hits.len(), 1);
        assert_eq!(title_hits[0].match_field, SearchMatchField::Title);

        let text_hits = search_conversations(&vault, "Revenue").expect("search");
        assert!(
            text_hits
                .iter()
                .any(|hit| hit.match_field == SearchMatchField::Text)
        );

        let thinking_hits = search_conversations(&vault, "CSV source").expect("search");
        assert!(
            thinking_hits
                .iter()
                .any(|hit| hit.match_field == SearchMatchField::Thinking)
        );

        let tool_hits = search_conversations(&vault, "hidden keyword").expect("search");
        assert!(tool_hits.is_empty());
    }

    #[test]
    fn search_empty_state_names_scope() {
        assert!(SEARCH_SCOPE_MESSAGE.contains("Text"));
        assert!(SEARCH_SCOPE_MESSAGE.contains("Thinking"));
        assert!(SEARCH_SCOPE_MESSAGE.contains("titles"));
    }
}
