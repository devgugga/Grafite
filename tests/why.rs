mod support;

use std::process::Command;

fn bin(repo: &std::path::Path) -> Command {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_grafite"));
    cmd.current_dir(repo);
    cmd
}

#[test]
fn why_reports_the_rationale_of_commits_touching_a_path() {
    let repo = support::fixture_repo("why");
    support::commit_file(
        &repo,
        "cli/doctor.py",
        "print()",
        "✨ add doctor: cli\n\n### 💡 Architecture improvements\n- Introduced a non-mutating health check.\n\n### 🚀 Outcome\n- Works.",
    );
    assert!(bin(&repo)
        .arg("sync")
        .output()
        .expect("runs")
        .status
        .success());

    let output = bin(&repo)
        .args(["why", "cli/doctor.py"])
        .output()
        .expect("runs");
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let payload: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("stdout is pure JSON");
    let records = payload["records"].as_array().expect("records array");
    assert_eq!(records.len(), 1);
    assert_eq!(
        records[0]["rationale"]["architecture"][0],
        "Introduced a non-mutating health check."
    );
}

#[test]
fn why_without_sync_is_an_actionable_error_not_an_empty_result() {
    let repo = support::fixture_repo("why-unsynced");
    support::commit_file(
        &repo,
        "a.rs",
        "fn main() {}",
        "✨ add a: core\n\n### 🚀 Outcome\n- Added.",
    );

    let output = bin(&repo).args(["why", "a.rs"]).output().expect("runs");
    assert!(
        !output.status.success(),
        "must fail rather than report nothing"
    );
    assert!(output.stdout.is_empty(), "stdout must stay clean on error");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("grafite sync"),
        "must name the remediation: {stderr}"
    );
}
