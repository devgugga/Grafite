use crate::error::{Error, Result};
use serde::{Deserialize, Serialize};

pub const SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Kind {
    Commit,
    Spec,
    Plan,
    Adr,
}

/// The artifact an edge points at. Classification is by path, never by content.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Target {
    Spec(String),
    Plan(String),
    Adr(String),
    File(String),
}

impl Target {
    pub fn id(&self) -> String {
        match self {
            Target::Spec(p) => format!("spec:{p}"),
            Target::Plan(p) => format!("plan:{p}"),
            Target::Adr(p) => format!("adr:{p}"),
            Target::File(p) => format!("file:{p}"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EdgeType {
    #[serde(rename = "TOUCHES")]
    Touches,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Edge {
    #[serde(rename = "type")]
    pub edge_type: EdgeType,
    pub to: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Rationale {
    pub new_features: Vec<String>,
    pub architecture: Vec<String>,
    pub validations: Vec<String>,
    pub security: Vec<String>,
    pub outcome: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Record {
    pub schema_version: u32,
    pub id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub short_id: Option<String>,
    pub kind: Kind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gitmoji: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub subject: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scope: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub authored_at: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rationale: Option<Rationale>,
    pub edges: Vec<Edge>,
}

pub fn to_jsonl(records: &[Record]) -> Result<String> {
    let mut out = String::new();
    for record in records {
        let line = serde_json::to_string(record).map_err(|e| {
            Error::new(
                "serialize provenance record",
                record.id.clone(),
                e.to_string(),
                "report this as a Grafite defect",
            )
        })?;
        out.push_str(&line);
        out.push('\n');
    }
    Ok(out)
}

pub fn from_jsonl(text: &str) -> Result<Vec<Record>> {
    let mut records = Vec::new();
    for (index, line) in text.lines().enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        let record = serde_json::from_str(line).map_err(|e| {
            Error::new(
                "parse provenance record",
                format!("line {}", index + 1),
                e.to_string(),
                "regenerate the records with `grafite sync`",
            )
        })?;
        records.push(record);
    }
    Ok(records)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> Record {
        Record {
            schema_version: SCHEMA_VERSION,
            id: "commit:4bc618c55e888ceb468e06fe89597c42ff052751".to_string(),
            short_id: Some("4bc618c5".to_string()),
            kind: Kind::Commit,
            gitmoji: Some("✨".to_string()),
            subject: Some("diagnose secret service".to_string()),
            scope: Some("cli".to_string()),
            authored_at: Some("2026-09-06T14:21:03+00:00".to_string()),
            rationale: Some(Rationale::default()),
            edges: vec![Edge {
                edge_type: EdgeType::Touches,
                to: Target::File("cli/asb/doctor.py".to_string()).id(),
            }],
        }
    }

    #[test]
    fn target_ids_are_prefixed_by_kind() {
        assert_eq!(Target::File("a/b.rs".into()).id(), "file:a/b.rs");
        assert_eq!(Target::Spec("docs/s.md".into()).id(), "spec:docs/s.md");
        assert_eq!(Target::Plan("docs/p.md".into()).id(), "plan:docs/p.md");
        assert_eq!(Target::Adr("docs/a.md".into()).id(), "adr:docs/a.md");
    }

    #[test]
    fn jsonl_round_trips_and_is_one_line_per_record() {
        let rendered = to_jsonl(&[sample(), sample()]).expect("serializes");
        assert_eq!(rendered.lines().count(), 2);
        let parsed = from_jsonl(&rendered).expect("parses");
        assert_eq!(parsed.len(), 2);
        assert_eq!(parsed[0].id, sample().id);
        assert_eq!(parsed[0].edges[0].to, "file:cli/asb/doctor.py");
    }

    #[test]
    fn serialization_is_byte_stable_across_runs() {
        assert_eq!(
            to_jsonl(&[sample()]).expect("first"),
            to_jsonl(&[sample()]).expect("second")
        );
    }

    #[test]
    fn edge_type_serializes_as_touches() {
        let rendered = to_jsonl(&[sample()]).expect("serializes");
        assert!(rendered.contains("\"TOUCHES\""), "got: {rendered}");
    }
}
