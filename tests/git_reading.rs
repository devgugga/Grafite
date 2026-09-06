mod support;

#[test]
fn reads_commits_with_touched_paths() {
    let repo = support::fixture_repo("git-reading");
    support::commit_file(
        &repo,
        "a.txt",
        "one",
        "✨ add a: core\n\n### 🚀 Outcome\n- done.",
    );
    support::commit_file(
        &repo,
        "b.txt",
        "two",
        "🐛 fix b: core\n\n### 🚀 Outcome\n- fixed.",
    );

    let commits = grafite::git::read_commits(&repo).expect("reads commits");

    assert_eq!(commits.len(), 2);
    assert_eq!(commits[0].subject, "🐛 fix b: core");
    assert_eq!(commits[0].touched, vec!["b.txt".to_string()]);
    assert_eq!(commits[1].touched, vec!["a.txt".to_string()]);
    assert_eq!(
        commits[0].sha.len(),
        40,
        "canonical identity is the full SHA"
    );
    assert!(commits[0].body.contains("### 🚀 Outcome"));
}
