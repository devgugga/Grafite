use std::path::{Path, PathBuf};
use std::process::Command;

/// Build a disposable git repository under target/ for end-to-end tests.
pub fn fixture_repo(name: &str) -> PathBuf {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("target")
        .join("test-repos")
        .join(name);
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).expect("create fixture root");
    run(&root, &["init", "--initial-branch=main"]);
    run(&root, &["config", "user.email", "fixture@example.com"]);
    run(&root, &["config", "user.name", "Fixture"]);
    root
}

pub fn run(repo: &Path, args: &[&str]) {
    let status = Command::new("git")
        .arg("-C")
        .arg(repo)
        .args(args)
        .status()
        .expect("git runs");
    assert!(status.success(), "git {args:?} failed");
}

/// Write a file and commit it with the given message.
pub fn commit_file(repo: &Path, path: &str, contents: &str, message: &str) {
    let full = repo.join(path);
    if let Some(parent) = full.parent() {
        std::fs::create_dir_all(parent).expect("create parent");
    }
    std::fs::write(&full, contents).expect("write file");
    run(repo, &["add", path]);
    let status = Command::new("git")
        .arg("-C")
        .arg(repo)
        .args(["commit", "-m", message])
        .env("GIT_AUTHOR_DATE", "2026-01-01T00:00:00+00:00")
        .env("GIT_COMMITTER_DATE", "2026-01-01T00:00:00+00:00")
        .status()
        .expect("git commit runs");
    assert!(status.success(), "commit failed");
}
