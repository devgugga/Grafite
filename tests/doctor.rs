mod support;

use std::process::Command;

#[test]
fn doctor_reports_absent_providers_as_healthy() {
    let repo = support::fixture_repo("doctor");
    support::commit_file(
        &repo,
        "a.rs",
        "fn main() {}",
        "✨ add a: core\n\n### 🚀 Outcome\n- Added.",
    );

    let output = Command::new(env!("CARGO_BIN_EXE_grafite"))
        .current_dir(&repo)
        .arg("doctor")
        .output()
        .expect("runs");

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let payload: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("stdout is pure JSON");

    assert_eq!(payload["healthy"], true);
    let providers = payload["providers"].as_array().expect("providers array");
    assert_eq!(
        providers[0]["provider"], "graphify",
        "providers must be sorted"
    );
    assert_eq!(providers[1]["provider"], "semantica");
    let semantica_status = &providers[1]["status"];
    assert!(
        semantica_status == "absent" || semantica_status == "ok",
        "unexpected status: {semantica_status}"
    );
}
