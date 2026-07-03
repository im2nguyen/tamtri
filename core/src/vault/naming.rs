use chrono::Datelike;

use crate::conversation::Conversation;

pub fn folder_name(c: &Conversation) -> String {
    let date = c.created_at.date_naive();
    format!(
        "{:04}-{:02}-{:02}-{}--{}",
        date.year(),
        date.month(),
        date.day(),
        slug(&c.title),
        c.id.simple()
    )
}

pub fn slug(input: &str) -> String {
    let mut out = String::new();
    let mut previous_dash = false;

    for ch in input.chars().flat_map(char::to_lowercase) {
        let ascii = match ch {
            'a'..='z' | '0'..='9' => Some(ch),
            _ if ch.is_ascii_alphanumeric() => Some(ch),
            _ => None,
        };

        if let Some(ch) = ascii {
            out.push(ch);
            previous_dash = false;
        } else if !previous_dash && !out.is_empty() {
            out.push('-');
            previous_dash = true;
        }

        if out.len() >= 40 {
            break;
        }
    }

    let trimmed = out.trim_matches('-').to_string();
    if trimmed.is_empty() {
        "untitled".to_string()
    } else {
        trimmed
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slug_truncates_to_forty_chars() {
        let long = "a".repeat(100);
        assert_eq!(slug(&long).len(), 40);
        assert_eq!(slug(&long), "a".repeat(40));
    }

    #[test]
    fn slug_untitled_for_empty_input() {
        assert_eq!(slug(""), "untitled");
        assert_eq!(slug("   "), "untitled");
        assert_eq!(slug("!!!"), "untitled");
        assert_eq!(slug("---"), "untitled");
    }
}
