mod support;

use std::process::Command;

fn bin(repo: &std::path::Path) -> Command {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_grafite"));
    cmd.current_dir(repo);
    cmd
}

#[test]
fn sync_writes_records_and_is_byte_identical_across_runs() {
    let repo = support::fixture_repo("sync");
    support::commit_file(
        &repo,
        "cli/doctor.py",
        "print()",
        "✨ add doctor: cli\n\n### ✅ New features\n- Added a doctor.\n\n### 🚀 Outcome\n- Works.",
    );
    support::commit_file(
        &repo,
        "graphify-out/graph.json",
        "{}",
        "🕸️ sync knowledge graph",
    );

    let first = bin(&repo).arg("sync").output().expect("runs");
    assert!(
        first.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&first.stderr)
    );

    let records = repo.join(".grafite/state/records/decisions.jsonl");
    let after_first = std::fs::read(&records).expect("records written");

    let second = bin(&repo).arg("sync").output().expect("runs");
    assert!(second.status.success());
    let after_second = std::fs::read(&records).expect("records still there");

    assert_eq!(
        after_first, after_second,
        "sync must be byte-identical across runs"
    );

    let text = String::from_utf8(after_first).expect("utf8");
    assert_eq!(
        text.lines().count(),
        1,
        "graph-sync commit must be discarded"
    );
    assert!(text.contains("\"file:cli/doctor.py\""));
    assert!(text.contains("\"Added a doctor.\""));
    assert!(
        !text.contains("graphify-out"),
        "discarded commit must leave no trace"
    );
}

#[test]
fn sync_classifies_spec_targets_so_traversal_resolves() {
    let repo = support::fixture_repo("sync-spec");
    support::commit_file(
        &repo,
        "docs/superpowers/specs/2026-01-01-x-design.md",
        "# x",
        "📝 add spec: design\n\n### 🚀 Outcome\n- Spec added.",
    );

    assert!(bin(&repo)
        .arg("sync")
        .output()
        .expect("runs")
        .status
        .success());

    let text = std::fs::read_to_string(repo.join(".grafite/state/records/decisions.jsonl"))
        .expect("records");
    assert!(
        text.contains("\"spec:docs/superpowers/specs/2026-01-01-x-design.md\""),
        "spec target must carry the spec: prefix, got {text}"
    );
}

#[test]
fn sync_omits_excluded_paths() {
    let repo = support::fixture_repo("sync-excluded");
    support::commit_file(
        &repo,
        ".env",
        "SECRET=1",
        "🔧 add env: config\n\n### 🚀 Outcome\n- Added.",
    );

    assert!(bin(&repo)
        .arg("sync")
        .output()
        .expect("runs")
        .status
        .success());

    let text = std::fs::read_to_string(repo.join(".grafite/state/records/decisions.jsonl"))
        .expect("records");
    assert!(!text.contains(".env"), "excluded path leaked: {text}");
}
