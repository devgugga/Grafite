use crate::record::Rationale;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedTitle {
    pub gitmoji: Option<String>,
    pub subject: String,
    pub scope: Option<String>,
}

/// Title shape from commit-conventions.md 4.4: `<gitmoji> <outcome>: <context>`.
pub fn parse_title(title: &str) -> ParsedTitle {
    let trimmed = title.trim();
    let mut rest = trimmed;
    let mut gitmoji = None;

    if let Some((first, remainder)) = trimmed.split_once(' ') {
        // A leading token containing no ASCII alphanumeric is treated as the gitmoji.
        if !first.is_empty() && !first.chars().any(|c| c.is_ascii_alphanumeric()) {
            gitmoji = Some(first.to_string());
            rest = remainder.trim_start();
        }
    }

    match rest.rsplit_once(": ") {
        Some((subject, scope)) => ParsedTitle {
            gitmoji,
            subject: subject.trim().to_string(),
            scope: Some(scope.trim().to_string()),
        },
        None => ParsedTitle {
            gitmoji,
            subject: rest.to_string(),
            scope: None,
        },
    }
}

pub fn is_graph_sync(subject: &str) -> bool {
    subject.trim() == "🕸️ sync knowledge graph"
}

const HEADINGS: [(&str, usize); 5] = [
    ("### ✅ New features", 0),
    ("### 💡 Architecture improvements", 1),
    ("### 🧼 Best practices & validations", 2),
    ("### 🔐 Security & Access Control", 3),
    ("### 🚀 Outcome", 4),
];

/// Split a structured body into its five sections. A body without headings
/// yields an empty rationale; that is a valid outcome, not a parse failure.
pub fn parse_rationale(body: &str) -> Rationale {
    let mut buckets: [Vec<String>; 5] = Default::default();
    let mut current: Option<usize> = None;

    for line in body.lines() {
        let trimmed = line.trim_end();
        if let Some(index) = heading_index(trimmed.trim()) {
            current = Some(index);
            continue;
        }
        let Some(index) = current else { continue };
        let content = trimmed.trim_start();
        if content.is_empty() {
            continue;
        }
        // SAFETY OF INDEXING: `index` originates only from `heading_index`,
        // which returns a value taken from `HEADINGS`. That array holds five
        // entries with indices 0..=4, and `buckets` is `[Vec<String>; 5]`, so
        // `buckets[index]` is always in bounds.
        if let Some(bullet) = content.strip_prefix("- ") {
            buckets[index].push(bullet.trim().to_string());
        } else if buckets[index].is_empty() {
            buckets[index].push(content.to_string());
        } else if let Some(last) = buckets[index].last_mut() {
            // Continuation of the previous bullet, wrapped at 76 columns.
            last.push(' ');
            last.push_str(content);
        }
    }

    let [new_features, architecture, validations, security, outcome] = buckets;
    Rationale {
        new_features,
        architecture,
        validations,
        security,
        outcome,
    }
}

fn heading_index(line: &str) -> Option<usize> {
    HEADINGS
        .iter()
        .find(|(heading, _)| *heading == line)
        .map(|(_, index)| *index)
}

#[cfg(test)]
mod tests {
    use super::*;

    const REAL_BODY: &str = "\
### ✅ New features
- Added diagnostic check in cli/asb/doctor.py for the Secret Service
  singleton (asb-keyring).

### 💡 Architecture improvements
- Replaced custom polling loop with check_keyring_service().

### 🧼 Best practices & validations
- Added unit tests in tests/unit/test_doctor.py.

### 🚀 Outcome
- Doctor diagnoses the Secret Service without mutating state.";

    #[test]
    fn splits_title_into_gitmoji_subject_and_scope() {
        let parsed = parse_title("✨ diagnose secret service in doctor: cli");
        assert_eq!(parsed.gitmoji.as_deref(), Some("✨"));
        assert_eq!(parsed.subject, "diagnose secret service in doctor");
        assert_eq!(parsed.scope.as_deref(), Some("cli"));
    }

    #[test]
    fn title_without_scope_yields_no_scope() {
        let parsed = parse_title("🐛 fix the thing");
        assert_eq!(parsed.subject, "fix the thing");
        assert_eq!(parsed.scope, None);
    }

    #[test]
    fn parses_all_five_sections() {
        let rationale = parse_rationale(REAL_BODY);
        assert_eq!(rationale.new_features.len(), 1);
        assert!(rationale.new_features[0].contains("asb-keyring"));
        assert_eq!(rationale.architecture.len(), 1);
        assert_eq!(rationale.validations.len(), 1);
        assert!(rationale.security.is_empty());
        assert_eq!(rationale.outcome.len(), 1);
    }

    #[test]
    fn bullets_spanning_lines_are_joined() {
        let rationale = parse_rationale(REAL_BODY);
        assert!(
            rationale.new_features[0].contains("singleton (asb-keyring)"),
            "continuation line must join its bullet: {:?}",
            rationale.new_features[0]
        );
    }

    #[test]
    fn body_without_headings_yields_empty_rationale_not_an_error() {
        let rationale = parse_rationale("just some prose with no headings");
        assert_eq!(rationale, crate::record::Rationale::default());
    }

    #[test]
    fn recognizes_graph_sync_commits() {
        assert!(is_graph_sync("🕸️ sync knowledge graph"));
        assert!(!is_graph_sync("✨ add a thing: core"));
    }

    #[test]
    fn unbulleted_text_under_heading_is_captured() {
        let body = "\
### 🚀 Outcome
Initial unbulleted outcome sentence.
Continuation of unbulleted sentence.";
        let rationale = parse_rationale(body);
        assert_eq!(rationale.outcome.len(), 1);
        assert_eq!(
            rationale.outcome[0],
            "Initial unbulleted outcome sentence. Continuation of unbulleted sentence."
        );
    }
}
