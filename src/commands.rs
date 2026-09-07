use crate::error::{Error, Result};
use crate::extract::{is_graph_sync, parse_rationale, parse_title};
use crate::git::{self, RawCommit};
use crate::paths::{classify, is_excluded};
use crate::record::{self, Edge, EdgeType, Kind, Record, SCHEMA_VERSION};
use std::path::{Path, PathBuf};

pub fn records_path(repo: &Path) -> PathBuf {
    repo.join(".grafite")
        .join("state")
        .join("records")
        .join("decisions.jsonl")
}

fn short_id(sha: &str) -> Option<String> {
    // Fixed width: git's auto-abbreviation varies with repository size.
    sha.get(..8).map(str::to_string)
}

pub fn build_records(commits: &[RawCommit]) -> Vec<Record> {
    let mut records = Vec::new();
    for commit in commits {
        if is_graph_sync(&commit.subject) {
            continue;
        }
        let title = parse_title(&commit.subject);
        let mut edges: Vec<Edge> = commit
            .touched
            .iter()
            .filter(|path| !is_excluded(path))
            .map(|path| Edge {
                edge_type: EdgeType::Touches,
                to: classify(path).id(),
            })
            .collect();
        edges.sort_by(|a, b| a.to.cmp(&b.to));
        edges.dedup_by(|a, b| a.to == b.to);

        records.push(Record {
            schema_version: SCHEMA_VERSION,
            id: format!("commit:{}", commit.sha),
            short_id: short_id(&commit.sha),
            kind: Kind::Commit,
            gitmoji: title.gitmoji,
            subject: Some(title.subject),
            scope: title.scope,
            authored_at: Some(commit.authored_at.clone()),
            rationale: Some(parse_rationale(&commit.body)),
            edges,
        });
    }
    records
}

pub fn sync(repo: &Path) -> Result<String> {
    let commits = git::read_commits(repo)?;
    let records = build_records(&commits);
    let rendered = record::to_jsonl(&records)?;

    let destination = records_path(repo);
    if let Some(parent) = destination.parent() {
        std::fs::create_dir_all(parent).map_err(|e| {
            Error::new(
                "create the records directory",
                parent.display().to_string(),
                e.to_string(),
                "ensure the repository directory is writable",
            )
        })?;
    }
    std::fs::write(&destination, rendered).map_err(|e| {
        Error::new(
            "write provenance records",
            destination.display().to_string(),
            e.to_string(),
            "ensure the repository directory is writable",
        )
    })?;

    let summary = serde_json::json!({
        "schema_version": SCHEMA_VERSION,
        "records": records.len(),
        "commits_read": commits.len(),
    });
    Ok(summary.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn records_path_matches_expected() {
        let repo = Path::new("/workspace");
        assert_eq!(
            records_path(repo),
            PathBuf::from("/workspace/.grafite/state/records/decisions.jsonl")
        );
    }

    #[test]
    fn build_records_discards_graph_sync_and_dedups_edges() {
        let commits = vec![
            RawCommit {
                sha: "1234567890abcdef1234567890abcdef12345678".into(),
                authored_at: "2026-01-01T00:00:00+00:00".into(),
                subject: "✨ add feature: core".into(),
                body: "### 🚀 Outcome\n- Done.".into(),
                touched: vec!["src/main.rs".into(), "src/main.rs".into(), ".env".into()],
            },
            RawCommit {
                sha: "abcdef1234567890abcdef1234567890abcdef12".into(),
                authored_at: "2026-01-02T00:00:00+00:00".into(),
                subject: "🕸️ sync knowledge graph".into(),
                body: "".into(),
                touched: vec!["graphify-out/graph.json".into()],
            },
        ];

        let records = build_records(&commits);
        assert_eq!(records.len(), 1);
        assert_eq!(
            records[0].id,
            "commit:1234567890abcdef1234567890abcdef12345678"
        );
        assert_eq!(records[0].short_id.as_deref(), Some("12345678"));
        assert_eq!(records[0].gitmoji.as_deref(), Some("✨"));
        assert_eq!(records[0].edges.len(), 1);
        assert_eq!(records[0].edges[0].to, "file:src/main.rs");
    }
}
