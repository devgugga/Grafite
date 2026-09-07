use crate::paths::classify;
use crate::record::Record;

/// Records that touched `path`. Reports association only; asserts no causality.
pub fn why(records: &[Record], path: &str) -> Vec<Record> {
    let target = classify(path).id();
    let mut found: Vec<Record> = records
        .iter()
        .filter(|record| record.edges.iter().any(|edge| edge.to == target))
        .cloned()
        .collect();
    found.sort_by(|a, b| a.id.cmp(&b.id));
    found
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::record::{Edge, EdgeType, Kind, Record, SCHEMA_VERSION};

    fn record_with(id: &str, target: &str) -> Record {
        Record {
            schema_version: SCHEMA_VERSION,
            id: id.to_string(),
            short_id: None,
            kind: Kind::Commit,
            gitmoji: None,
            subject: None,
            scope: None,
            authored_at: None,
            rationale: None,
            edges: vec![Edge {
                edge_type: EdgeType::Touches,
                to: target.to_string(),
            }],
        }
    }

    #[test]
    fn matches_records_touching_the_path_and_sorts_by_id() {
        let records = vec![
            record_with("commit:bbb", "file:a.rs"),
            record_with("commit:aaa", "file:a.rs"),
            record_with("commit:ccc", "file:other.rs"),
        ];
        let found = why(&records, "a.rs");
        assert_eq!(found.len(), 2);
        assert_eq!(found[0].id, "commit:aaa");
        assert_eq!(found[1].id, "commit:bbb");
    }

    #[test]
    fn matches_a_spec_path_through_its_classified_target() {
        let path = "docs/superpowers/specs/2026-01-01-x-design.md";
        let records = vec![record_with("commit:aaa", &format!("spec:{path}"))];
        assert_eq!(why(&records, path).len(), 1);
    }

    #[test]
    fn unknown_path_yields_no_matches() {
        let records = vec![record_with("commit:aaa", "file:a.rs")];
        assert!(why(&records, "nope.rs").is_empty());
    }
}
