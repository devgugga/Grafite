# Grafite MVP Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build the Grafite CLI so that `sync`, `doctor`, and `why` extract and query canonical provenance records from verifiable Git facts.

**Architecture:** A Rust binary reads Git through the `git` subprocess, parses structured commit bodies into provenance records, writes them as JSONL under `.grafite/state/records/`, and answers `why <path>` natively from those records. No provider is invoked; `doctor` only detects and delegates.

**Tech Stack:** Rust 2021, `clap` (derive), `serde`, `serde_json`. Git via subprocess — no `libgit2`. End-to-end tests drive the real binary through `env!("CARGO_BIN_EXE_grafite")`, so no test-harness crate is added.

**Spec:** `docs/superpowers/specs/2026-09-06-grafite-mvp-design.md`

## Global Constraints

- **Dependencies are frozen at three:** `clap`, `serde`, `serde_json`. Adding any other crate requires human approval per `docs/domains/rust/conventions.md` §3.1.
- **Deviation from spec §3, pending approval:** this plan does **not** read `.grafite/config.toml`. Reading TOML would require a fourth crate, and in the MVP there is no provider to enable and the globs have defaults. Globs are built-in constants (Task 5). Reversing this deviation means approving the `toml` crate and adding a config-loading task.
- **Stream discipline:** `stdout` carries only the payload. Every diagnostic, warning, and error goes to `stderr`. (`cli-contract.md` §1)
- **Exit codes:** `0` only when the command did what it was asked. Otherwise non-zero. (`cli-contract.md` §2)
- **Determinism:** every emitted collection is explicitly sorted. No absolute paths, no wall-clock timestamps, no hash-iteration order in output. (`cli-contract.md` §4)
- **`short_id` is exactly the first 8 characters of the SHA.** Never `git`'s auto-abbreviation, whose length varies with repository size and would break determinism.
- **Panic policy:** no `unwrap()`, `expect()`, `panic!()`, or panicking indexing in non-test code. Fallible operations return `Result`. (`conventions.md` §2)
- **Error text order:** failing operation, offending input, remediation — in that order. (`cli-contract.md` §3.4)
- **Toolchain gates**, run before every commit:
  ```bash
  cargo fmt --all -- --check
  cargo clippy --all-targets --all-features -- -D warnings
  cargo test --all-features
  ```
- **Commits** follow `docs/domains/git/commit-conventions.md`: literal gitmoji, structured body wrapped at 76 columns, no co-authorship trailers. The repository is on `main`; committing there requires human authorization each time.

## File Structure

| File | Responsibility |
| :--- | :--- |
| `Cargo.toml` | Crate manifest, three dependencies. |
| `src/main.rs` | CLI definition, exit codes, stream discipline. Thin. |
| `src/error.rs` | One error type carrying operation, input, cause, remediation. |
| `src/git.rs` | Reading commits and touched paths via the `git` subprocess. |
| `src/record.rs` | Record, edge, and rationale types; JSONL serialization. |
| `src/extract.rs` | Commit title and body to record. Pure, no I/O. |
| `src/paths.rs` | Target classification by glob; exclusion policy. Pure. |
| `src/query.rs` | Native `why` over loaded records. Pure. |
| `src/provider.rs` | Provider detection and delegation for `doctor`. |
| `src/commands.rs` | Orchestration of `sync`, `doctor`, `why`. |
| `tests/support/mod.rs` | Fixture-repository builder shared by end-to-end tests. |
| `tests/cli.rs` | End-to-end tests driving the real binary. |

`src/extract.rs`, `src/paths.rs`, and `src/query.rs` are pure so they can be unit-tested without a repository. All I/O concentrates in `git.rs`, `provider.rs`, and `commands.rs`.

---

### Task 1: Crate scaffold and CLI surface

**Files:**
- Create: `Cargo.toml`, `src/main.rs`, `src/error.rs`, `.gitignore`
- Test: `tests/cli.rs`

**Interfaces:**
- Consumes: nothing.
- Produces: binary `grafite` with subcommands `sync`, `doctor`, `why <path>`. `error::Error::new(operation, input, cause, remediation) -> Error`, `error::Result<T> = std::result::Result<T, Error>`.

- [ ] **Step 1: Write the failing test**

Create `tests/cli.rs`:

```rust
use std::process::Command;

fn bin() -> Command {
    Command::new(env!("CARGO_BIN_EXE_grafite"))
}

#[test]
fn why_requires_a_path_argument() {
    let output = bin().arg("why").output().expect("binary runs");
    assert!(!output.status.success(), "missing argument must fail");
    assert!(output.stdout.is_empty(), "stdout must stay clean on error");
}

#[test]
fn unknown_subcommand_fails_without_touching_stdout() {
    let output = bin().arg("nonesuch").output().expect("binary runs");
    assert!(!output.status.success());
    assert!(output.stdout.is_empty(), "stdout must stay clean on error");
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --test cli`
Expected: FAIL — the crate does not exist yet (`error: failed to parse manifest` or no such binary).

- [ ] **Step 3: Write minimal implementation**

`Cargo.toml`:

```toml
[package]
name = "grafite"
version = "0.1.0"
edition = "2021"

[dependencies]
clap = { version = "4", features = ["derive"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
```

`.gitignore`:

```text
/target
/.grafite/state/
```

`src/error.rs`:

```rust
use std::fmt;

/// An error an agent can act on: what was attempted, on what input, and how to fix it.
#[derive(Debug)]
pub struct Error {
    operation: String,
    input: String,
    cause: String,
    remediation: String,
}

impl Error {
    pub fn new(
        operation: impl Into<String>,
        input: impl Into<String>,
        cause: impl Into<String>,
        remediation: impl Into<String>,
    ) -> Self {
        Self {
            operation: operation.into(),
            input: input.into(),
            cause: cause.into(),
            remediation: remediation.into(),
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // cli-contract.md 3.4: operation, offending input, remediation, in that order.
        write!(
            f,
            "failed to {}: input {}: {}: {}",
            self.operation, self.input, self.cause, self.remediation
        )
    }
}

impl std::error::Error for Error {}

pub type Result<T> = std::result::Result<T, Error>;
```

`src/main.rs`:

```rust
mod error;

use clap::{Parser, Subcommand};
use std::process::ExitCode;

#[derive(Parser)]
#[command(name = "grafite", version, about = "Provenance records from verifiable Git facts")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Extract provenance records from Git history.
    Sync,
    /// Report provider preconditions.
    Doctor,
    /// Show provenance records that touched a path.
    Why { path: String },
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    let outcome: error::Result<String> = match cli.command {
        Command::Sync => Ok(String::new()),
        Command::Doctor => Ok(String::new()),
        Command::Why { path: _ } => Ok(String::new()),
    };
    match outcome {
        Ok(payload) => {
            if !payload.is_empty() {
                println!("{payload}");
            }
            ExitCode::SUCCESS
        }
        Err(err) => {
            eprintln!("{err}");
            ExitCode::from(1)
        }
    }
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test --test cli`
Expected: PASS, both tests.

- [ ] **Step 5: Run the toolchain gates**

Run: `cargo fmt --all -- --check && cargo clippy --all-targets --all-features -- -D warnings && cargo test --all-features`
Expected: all three succeed. Report output verbatim; do not proceed on failure.

- [ ] **Step 6: Commit**

```bash
git add Cargo.toml Cargo.lock .gitignore src/ tests/
git commit
```

Title: `✨ scaffold grafite cli surface and error contract: cli`
Body must include `### ✅ New features` and `### 🚀 Outcome`, wrapped at 76 columns.

---

### Task 2: Git reading layer

**Files:**
- Create: `src/git.rs`, `tests/support/mod.rs`
- Modify: `src/main.rs` (add `mod git;`)

**Interfaces:**
- Consumes: `error::{Error, Result}`.
- Produces: `git::RawCommit { sha: String, authored_at: String, subject: String, body: String, touched: Vec<String> }` and `git::read_commits(repo: &Path) -> Result<Vec<RawCommit>>`, returning commits in `git log` order (newest first) with `touched` sorted.

- [ ] **Step 1: Write the fixture helper**

Create `tests/support/mod.rs`:

```rust
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
```

- [ ] **Step 2: Write the failing test**

Create `tests/git_reading.rs`:

```rust
mod support;

#[test]
fn reads_commits_with_touched_paths() {
    let repo = support::fixture_repo("git-reading");
    support::commit_file(&repo, "a.txt", "one", "✨ add a: core\n\n### 🚀 Outcome\n- done.");
    support::commit_file(&repo, "b.txt", "two", "🐛 fix b: core\n\n### 🚀 Outcome\n- fixed.");

    let commits = grafite::git::read_commits(&repo).expect("reads commits");

    assert_eq!(commits.len(), 2);
    assert_eq!(commits[0].subject, "🐛 fix b: core");
    assert_eq!(commits[0].touched, vec!["b.txt".to_string()]);
    assert_eq!(commits[1].touched, vec!["a.txt".to_string()]);
    assert_eq!(commits[0].sha.len(), 40, "canonical identity is the full SHA");
    assert!(commits[0].body.contains("### 🚀 Outcome"));
}
```

This test uses the crate as a library, so the crate needs a `src/lib.rs`. Add it in Step 3.

- [ ] **Step 3: Run test to verify it fails**

Run: `cargo test --test git_reading`
Expected: FAIL — `use of undeclared crate or module 'grafite'`.

- [ ] **Step 4: Write minimal implementation**

Create `src/lib.rs` so the modules are testable from integration tests:

```rust
pub mod error;
pub mod git;
```

Change `src/main.rs` to consume the library instead of declaring modules: replace `mod error;` with `use grafite::error;`.

Create `src/git.rs`:

```rust
use crate::error::{Error, Result};
use std::path::Path;
use std::process::Command;

/// One commit as Git reports it, before any interpretation.
#[derive(Debug, Clone)]
pub struct RawCommit {
    pub sha: String,
    pub authored_at: String,
    pub subject: String,
    pub body: String,
    pub touched: Vec<String>,
}

// ASCII group/record separators cannot appear in a commit message written by
// the conventions, so they delimit fields without escaping.
const COMMIT_SEP: char = '\u{1d}';
const FIELD_SEP: char = '\u{1e}';

pub fn read_commits(repo: &Path) -> Result<Vec<RawCommit>> {
    let format = format!("{COMMIT_SEP}%H{FIELD_SEP}%aI{FIELD_SEP}%s{FIELD_SEP}%b{FIELD_SEP}");
    let output = Command::new("git")
        .arg("-C")
        .arg(repo)
        .args(["log", "--name-only"])
        .arg(format!("--format={format}"))
        .output()
        .map_err(|e| {
            Error::new(
                "run git log",
                repo.display().to_string(),
                e.to_string(),
                "install git and ensure it is on PATH",
            )
        })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(Error::new(
            "run git log",
            repo.display().to_string(),
            stderr,
            "run grafite inside a git repository with at least one commit",
        ));
    }

    let stdout = String::from_utf8(output.stdout).map_err(|e| {
        Error::new(
            "decode git log output",
            repo.display().to_string(),
            e.to_string(),
            "ensure commit messages are valid UTF-8",
        )
    })?;

    parse_log(&stdout, repo)
}

fn parse_log(stdout: &str, repo: &Path) -> Result<Vec<RawCommit>> {
    let mut commits = Vec::new();
    for chunk in stdout.split(COMMIT_SEP).skip(1) {
        let mut fields = chunk.split(FIELD_SEP);
        let sha = next_field(&mut fields, "sha", repo)?;
        let authored_at = next_field(&mut fields, "authored date", repo)?;
        let subject = next_field(&mut fields, "subject", repo)?;
        let body = next_field(&mut fields, "body", repo)?;
        let mut touched: Vec<String> = fields
            .next()
            .unwrap_or_default()
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty())
            .map(str::to_string)
            .collect();
        touched.sort();
        touched.dedup();
        commits.push(RawCommit {
            sha: sha.trim().to_string(),
            authored_at: authored_at.trim().to_string(),
            subject: subject.trim().to_string(),
            body: body.trim_end().to_string(),
            touched,
        });
    }
    Ok(commits)
}

fn next_field<'a>(
    fields: &mut impl Iterator<Item = &'a str>,
    what: &str,
    repo: &Path,
) -> Result<&'a str> {
    fields.next().ok_or_else(|| {
        Error::new(
            format!("parse {what} from git log output"),
            repo.display().to_string(),
            "field separator missing".to_string(),
            "report this as a Grafite defect; the git log format is internal",
        )
    })
}
```

- [ ] **Step 5: Run test to verify it passes**

Run: `cargo test --test git_reading`
Expected: PASS.

- [ ] **Step 6: Run the toolchain gates**

Run: `cargo fmt --all -- --check && cargo clippy --all-targets --all-features -- -D warnings && cargo test --all-features`
Expected: all succeed.

- [ ] **Step 7: Commit**

```bash
git add src/ tests/
git commit
```

Title: `✨ read commits and touched paths from git: core`

---

### Task 3: Record types and JSONL serialization

**Files:**
- Create: `src/record.rs`
- Modify: `src/lib.rs` (add `pub mod record;`)
- Test: unit tests inside `src/record.rs`

**Interfaces:**
- Consumes: `error::{Error, Result}`.
- Produces:
  - `record::Kind` — `Commit`, `Spec`, `Plan`, `Adr`
  - `record::Target` — `Spec(String)`, `Plan(String)`, `Adr(String)`, `File(String)`, with `Target::id() -> String` producing `"spec:<path>"`, `"plan:<path>"`, `"adr:<path>"`, `"file:<path>"`
  - `record::Edge { edge_type: EdgeType, to: String }` where `EdgeType::Touches` serializes as `"TOUCHES"`
  - `record::Rationale { new_features, architecture, validations, security, outcome }`, all `Vec<String>`
  - `record::Record { schema_version: u32, id: String, short_id: Option<String>, kind: Kind, gitmoji: Option<String>, subject: Option<String>, scope: Option<String>, authored_at: Option<String>, rationale: Option<Rationale>, edges: Vec<Edge> }`
  - `record::to_jsonl(&[Record]) -> Result<String>` and `record::from_jsonl(&str) -> Result<Vec<Record>>`
  - `record::SCHEMA_VERSION: u32 = 1`

- [ ] **Step 1: Write the failing test**

Append to `src/record.rs` (written in Step 3, tests written first here for clarity — create the file with only this test module to see it fail):

```rust
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
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --lib record`
Expected: FAIL — types not defined.

- [ ] **Step 3: Write minimal implementation**

Prepend to `src/record.rs`:

```rust
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
```

Add `pub mod record;` to `src/lib.rs`.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test --lib record`
Expected: PASS, four tests.

- [ ] **Step 5: Run the toolchain gates**

Run: `cargo fmt --all -- --check && cargo clippy --all-targets --all-features -- -D warnings && cargo test --all-features`

- [ ] **Step 6: Commit**

```bash
git add src/
git commit
```

Title: `✨ define provenance record schema and jsonl encoding: core`

---

### Task 4: Commit title and body parsing

**Files:**
- Create: `src/extract.rs`
- Modify: `src/lib.rs` (add `pub mod extract;`)
- Test: unit tests inside `src/extract.rs`

**Interfaces:**
- Consumes: `record::Rationale`.
- Produces:
  - `extract::ParsedTitle { gitmoji: Option<String>, subject: String, scope: Option<String> }`
  - `extract::parse_title(&str) -> ParsedTitle`
  - `extract::parse_rationale(&str) -> Rationale`
  - `extract::is_graph_sync(subject: &str) -> bool`

- [ ] **Step 1: Write the failing test**

Create `src/extract.rs` containing only:

```rust
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
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --lib extract`
Expected: FAIL — `parse_title`, `parse_rationale`, `is_graph_sync` not found.

- [ ] **Step 3: Write minimal implementation**

Prepend to `src/extract.rs`:

```rust
use crate::record::Rationale;

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
        if let Some(bullet) = content.strip_prefix("- ") {
            buckets[index].push(bullet.trim().to_string());
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
```

Add `pub mod extract;` to `src/lib.rs`.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test --lib extract`
Expected: PASS, six tests.

- [ ] **Step 5: Run the toolchain gates**

Run: `cargo fmt --all -- --check && cargo clippy --all-targets --all-features -- -D warnings && cargo test --all-features`

- [ ] **Step 6: Commit**

```bash
git add src/
git commit
```

Title: `✨ parse structured commit titles and bodies: core`

---

### Task 5: Path classification and exclusion policy

**Files:**
- Create: `src/paths.rs`
- Modify: `src/lib.rs` (add `pub mod paths;`)
- Test: unit tests inside `src/paths.rs`

**Interfaces:**
- Consumes: `record::Target`.
- Produces:
  - `paths::SPEC_PREFIX: &str = "docs/superpowers/specs/"`
  - `paths::PLAN_PREFIX: &str = "docs/superpowers/plans/"`
  - `paths::classify(path: &str) -> Target`
  - `paths::is_excluded(path: &str) -> bool`

- [ ] **Step 1: Write the failing test**

Create `src/paths.rs` containing only:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::record::Target;

    #[test]
    fn classifies_specs_and_plans_by_prefix() {
        assert_eq!(
            classify("docs/superpowers/specs/2026-09-06-x-design.md"),
            Target::Spec("docs/superpowers/specs/2026-09-06-x-design.md".into())
        );
        assert_eq!(
            classify("docs/superpowers/plans/2026-09-06-x.md"),
            Target::Plan("docs/superpowers/plans/2026-09-06-x.md".into())
        );
    }

    #[test]
    fn classifies_everything_else_as_file() {
        assert_eq!(
            classify("cli/asb/doctor.py"),
            Target::File("cli/asb/doctor.py".into())
        );
        // A non-markdown file inside the specs directory is still a file.
        assert_eq!(
            classify("docs/superpowers/specs/diagram.png"),
            Target::File("docs/superpowers/specs/diagram.png".into())
        );
    }

    #[test]
    fn excludes_secret_bearing_paths() {
        for path in [
            ".env",
            ".env.local",
            "image/.env.production",
            "keyring.pass",
            "secrets/db.pass",
            "home/.ssh/id_ed25519",
            "home/.ssh/id_ed25519.pub",
            ".agent-sandbox.toml",
            "state/runtime.json",
            "scratch/tmp.txt",
        ] {
            assert!(is_excluded(path), "must exclude {path}");
        }
    }

    #[test]
    fn does_not_exclude_ordinary_paths() {
        for path in ["cli/asb/doctor.py", "docs/domains/rust/conventions.md", "AGENTS.md"] {
            assert!(!is_excluded(path), "must not exclude {path}");
        }
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --lib paths`
Expected: FAIL — `classify` and `is_excluded` not found.

- [ ] **Step 3: Write minimal implementation**

Prepend to `src/paths.rs`:

```rust
use crate::record::Target;

// Built-in document sources. The spec allows overriding these through
// .grafite/config.toml; reading that file is deferred, see the plan's
// Global Constraints.
pub const SPEC_PREFIX: &str = "docs/superpowers/specs/";
pub const PLAN_PREFIX: &str = "docs/superpowers/plans/";

/// Classify an edge target by path. Reads no file content.
pub fn classify(path: &str) -> Target {
    let is_markdown = path.ends_with(".md");
    if is_markdown && path.starts_with(SPEC_PREFIX) {
        return Target::Spec(path.to_string());
    }
    if is_markdown && path.starts_with(PLAN_PREFIX) {
        return Target::Plan(path.to_string());
    }
    Target::File(path.to_string())
}

/// Mirrors the exclusion policy encoded in agent-sandbox's
/// .graphify/project.py, so exclusions stay consistent across providers.
const EXCLUDED_DIRECTORIES: [&str; 2] = ["state", "scratch"];

pub fn is_excluded(path: &str) -> bool {
    let mut components = path.split('/').peekable();
    let mut file_name = "";
    for component in components.by_ref() {
        if EXCLUDED_DIRECTORIES.contains(&component) {
            return true;
        }
        file_name = component;
    }

    file_name.starts_with(".env")
        || file_name.starts_with("id_ed25519")
        || file_name.starts_with(".agent-sandbox.toml")
        || file_name.ends_with(".pass")
        || file_name == "keyring.pass"
}
```

Add `pub mod paths;` to `src/lib.rs`.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test --lib paths`
Expected: PASS, four tests.

- [ ] **Step 5: Run the toolchain gates**

Run: `cargo fmt --all -- --check && cargo clippy --all-targets --all-features -- -D warnings && cargo test --all-features`

- [ ] **Step 6: Commit**

```bash
git add src/
git commit
```

Title: `🔐 classify edge targets and exclude secret paths: core`

Body must include `### 🔐 Security & Access Control` describing the mirrored exclusion policy.

---

### Task 6: `grafite sync`

**Files:**
- Create: `src/commands.rs`
- Modify: `src/lib.rs` (add `pub mod commands;`), `src/main.rs` (dispatch `Sync`)
- Test: `tests/sync.rs`

**Interfaces:**
- Consumes: `git::read_commits`, `extract::*`, `paths::*`, `record::*`.
- Produces:
  - `commands::records_path(repo: &Path) -> PathBuf` returning `<repo>/.grafite/state/records/decisions.jsonl`
  - `commands::build_records(commits: &[git::RawCommit]) -> Vec<record::Record>`
  - `commands::sync(repo: &Path) -> Result<String>` writing the file and returning a JSON summary payload

- [ ] **Step 1: Write the failing test**

Create `tests/sync.rs`:

```rust
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
    support::commit_file(&repo, "graphify-out/graph.json", "{}", "🕸️ sync knowledge graph");

    let first = bin(&repo).arg("sync").output().expect("runs");
    assert!(first.status.success(), "stderr: {}", String::from_utf8_lossy(&first.stderr));

    let records = repo.join(".grafite/state/records/decisions.jsonl");
    let after_first = std::fs::read(&records).expect("records written");

    let second = bin(&repo).arg("sync").output().expect("runs");
    assert!(second.status.success());
    let after_second = std::fs::read(&records).expect("records still there");

    assert_eq!(after_first, after_second, "sync must be byte-identical across runs");

    let text = String::from_utf8(after_first).expect("utf8");
    assert_eq!(text.lines().count(), 1, "graph-sync commit must be discarded");
    assert!(text.contains("\"file:cli/doctor.py\""));
    assert!(text.contains("\"Added a doctor.\""));
    assert!(!text.contains("graphify-out"), "discarded commit must leave no trace");
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

    assert!(bin(&repo).arg("sync").output().expect("runs").status.success());

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
    support::commit_file(&repo, ".env", "SECRET=1", "🔧 add env: config\n\n### 🚀 Outcome\n- Added.");

    assert!(bin(&repo).arg("sync").output().expect("runs").status.success());

    let text = std::fs::read_to_string(repo.join(".grafite/state/records/decisions.jsonl"))
        .expect("records");
    assert!(!text.contains(".env"), "excluded path leaked: {text}");
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --test sync`
Expected: FAIL — no records file is written.

- [ ] **Step 3: Write minimal implementation**

Create `src/commands.rs`:

```rust
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
```

Add `pub mod commands;` to `src/lib.rs`. In `src/main.rs`, replace the `Command::Sync` arm:

```rust
Command::Sync => match std::env::current_dir() {
    Ok(dir) => grafite::commands::sync(&dir),
    Err(e) => Err(grafite::error::Error::new(
        "determine the current directory",
        "<cwd>",
        e.to_string(),
        "run grafite from inside a git repository",
    )),
},
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test --test sync`
Expected: PASS, three tests.

- [ ] **Step 5: Run the toolchain gates**

Run: `cargo fmt --all -- --check && cargo clippy --all-targets --all-features -- -D warnings && cargo test --all-features`

- [ ] **Step 6: Commit**

```bash
git add src/ tests/
git commit
```

Title: `✨ extract provenance records from git history: sync`

---

### Task 7: `grafite why <path>`

**Files:**
- Create: `src/query.rs`
- Modify: `src/lib.rs` (add `pub mod query;`), `src/commands.rs` (add `why`), `src/main.rs` (dispatch `Why`)
- Test: unit tests in `src/query.rs`, end-to-end in `tests/why.rs`

**Interfaces:**
- Consumes: `record::Record`, `paths::classify`.
- Produces:
  - `query::why(records: &[Record], path: &str) -> Vec<Record>` returning matching records sorted by `id`
  - `commands::why(repo: &Path, path: &str) -> Result<String>`

- [ ] **Step 1: Write the failing unit test**

Create `src/query.rs` containing only:

```rust
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
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --lib query`
Expected: FAIL — `why` not found.

- [ ] **Step 3: Write minimal implementation**

Prepend to `src/query.rs`:

```rust
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
```

Add `pub mod query;` to `src/lib.rs`. Add to `src/commands.rs`:

```rust
pub fn why(repo: &Path, path: &str) -> Result<String> {
    let source = records_path(repo);
    let text = std::fs::read_to_string(&source).map_err(|e| {
        Error::new(
            "read provenance records",
            source.display().to_string(),
            e.to_string(),
            "run `grafite sync` first to generate the records",
        )
    })?;
    let records = record::from_jsonl(&text)?;
    let found = crate::query::why(&records, path);
    let payload = serde_json::json!({
        "schema_version": SCHEMA_VERSION,
        "path": path,
        "records": found,
    });
    Ok(payload.to_string())
}
```

The explicit read-and-error is deliberate: a missing records file must be an
actionable error, never an empty result that reads as "nothing touched this".

In `src/main.rs`, replace the `Command::Why` arm:

```rust
Command::Why { path } => match std::env::current_dir() {
    Ok(dir) => grafite::commands::why(&dir, &path),
    Err(e) => Err(grafite::error::Error::new(
        "determine the current directory",
        "<cwd>",
        e.to_string(),
        "run grafite from inside a git repository",
    )),
},
```

- [ ] **Step 4: Write the failing end-to-end test**

Create `tests/why.rs`:

```rust
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
    assert!(bin(&repo).arg("sync").output().expect("runs").status.success());

    let output = bin(&repo).args(["why", "cli/doctor.py"]).output().expect("runs");
    assert!(output.status.success(), "stderr: {}", String::from_utf8_lossy(&output.stderr));

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
    support::commit_file(&repo, "a.rs", "fn main() {}", "✨ add a: core\n\n### 🚀 Outcome\n- Added.");

    let output = bin(&repo).args(["why", "a.rs"]).output().expect("runs");
    assert!(!output.status.success(), "must fail rather than report nothing");
    assert!(output.stdout.is_empty(), "stdout must stay clean on error");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("grafite sync"), "must name the remediation: {stderr}");
}
```

- [ ] **Step 5: Run tests to verify they pass**

Run: `cargo test --lib query && cargo test --test why`
Expected: PASS, five tests total.

- [ ] **Step 6: Run the toolchain gates**

Run: `cargo fmt --all -- --check && cargo clippy --all-targets --all-features -- -D warnings && cargo test --all-features`

- [ ] **Step 7: Commit**

```bash
git add src/ tests/
git commit
```

Title: `✨ answer provenance queries for a path: why`

---

### Task 8: `grafite doctor`

**Files:**
- Create: `src/provider.rs`
- Modify: `src/lib.rs` (add `pub mod provider;`), `src/commands.rs` (add `doctor`), `src/main.rs` (dispatch `Doctor`)
- Test: unit tests in `src/provider.rs`, end-to-end in `tests/doctor.rs`

**Interfaces:**
- Consumes: `error::Result`.
- Produces:
  - `provider::Status` — `Ok`, `Absent`, `Failed`
  - `provider::Report { provider: String, status: Status, detail: String }`
  - `provider::graphify(repo: &Path) -> Report`
  - `provider::semantica() -> Report`
  - `commands::doctor(repo: &Path) -> Result<String>`

`src/provider.rs` holds provider **detection only**. The adapter protocol is
specified in the design document, not encoded in Rust: with no adapter to
implement, protocol structs would be types no caller uses.

- [ ] **Step 1: Write the failing unit test**

Create `src/provider.rs` containing only:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn absent_semantica_is_ok_because_absence_is_the_normal_state() {
        let report = Report {
            provider: "semantica".to_string(),
            status: Status::Absent,
            detail: "not installed".to_string(),
        };
        assert!(report.is_acceptable(), "absence must not be a failure");
    }

    #[test]
    fn failed_provider_is_not_acceptable() {
        let report = Report {
            provider: "graphify".to_string(),
            status: Status::Failed,
            detail: "version mismatch".to_string(),
        };
        assert!(!report.is_acceptable());
    }

    #[test]
    fn statuses_serialize_in_lowercase() {
        let json = serde_json::to_string(&Status::Absent).expect("serializes");
        assert_eq!(json, "\"absent\"");
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --lib provider`
Expected: FAIL — `Status` and `Report` not found.

- [ ] **Step 3: Write minimal implementation**

Prepend to `src/provider.rs`:

```rust
use serde::Serialize;
use std::path::Path;
use std::process::Command;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Status {
    Ok,
    Absent,
    Failed,
}

#[derive(Debug, Clone, Serialize)]
pub struct Report {
    pub provider: String,
    pub status: Status,
    pub detail: String,
}

impl Report {
    /// Absence is a normal state, not a warning: no MVP command needs a provider.
    pub fn is_acceptable(&self) -> bool {
        matches!(self.status, Status::Ok | Status::Absent)
    }
}

/// Delegate to Graphify's own verification rather than re-implementing the
/// pin, hook, and merge-driver checks, which would create a second definition
/// of Graphify health that silently drifts from the first.
pub fn graphify(repo: &Path) -> Report {
    let script = repo.join(".graphify").join("setup.sh");
    if !script.is_file() {
        return Report {
            provider: "graphify".to_string(),
            status: Status::Absent,
            detail: "no .graphify/setup.sh in this repository".to_string(),
        };
    }
    match Command::new(&script).arg("--verify-only").current_dir(repo).output() {
        Ok(output) if output.status.success() => Report {
            provider: "graphify".to_string(),
            status: Status::Ok,
            detail: String::from_utf8_lossy(&output.stdout).trim().to_string(),
        },
        Ok(output) => Report {
            provider: "graphify".to_string(),
            status: Status::Failed,
            detail: String::from_utf8_lossy(&output.stderr).trim().to_string(),
        },
        Err(e) => Report {
            provider: "graphify".to_string(),
            status: Status::Failed,
            detail: e.to_string(),
        },
    }
}

pub fn semantica() -> Report {
    match Command::new("semantica").arg("--version").output() {
        Ok(output) if output.status.success() => Report {
            provider: "semantica".to_string(),
            status: Status::Ok,
            detail: String::from_utf8_lossy(&output.stdout).trim().to_string(),
        },
        _ => Report {
            provider: "semantica".to_string(),
            status: Status::Absent,
            detail: "not installed; this is the expected state".to_string(),
        },
    }
}
```

Add `pub mod provider;` to `src/lib.rs`. Add to `src/commands.rs`:

```rust
pub fn doctor(repo: &Path) -> Result<String> {
    let mut reports = vec![crate::provider::graphify(repo), crate::provider::semantica()];
    reports.sort_by(|a, b| a.provider.cmp(&b.provider));
    let healthy = reports.iter().all(crate::provider::Report::is_acceptable);
    let payload = serde_json::json!({
        "schema_version": SCHEMA_VERSION,
        "healthy": healthy,
        "providers": reports,
    });
    Ok(payload.to_string())
}
```

In `src/main.rs`, replace the `Command::Doctor` arm:

```rust
Command::Doctor => match std::env::current_dir() {
    Ok(dir) => grafite::commands::doctor(&dir),
    Err(e) => Err(grafite::error::Error::new(
        "determine the current directory",
        "<cwd>",
        e.to_string(),
        "run grafite from inside a repository",
    )),
},
```

- [ ] **Step 4: Write the failing end-to-end test**

Create `tests/doctor.rs`:

```rust
mod support;

use std::process::Command;

#[test]
fn doctor_reports_absent_providers_as_healthy() {
    let repo = support::fixture_repo("doctor");
    support::commit_file(&repo, "a.rs", "fn main() {}", "✨ add a: core\n\n### 🚀 Outcome\n- Added.");

    let output = Command::new(env!("CARGO_BIN_EXE_grafite"))
        .current_dir(&repo)
        .arg("doctor")
        .output()
        .expect("runs");

    assert!(output.status.success(), "stderr: {}", String::from_utf8_lossy(&output.stderr));
    let payload: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("stdout is pure JSON");

    assert_eq!(payload["healthy"], true);
    let providers = payload["providers"].as_array().expect("providers array");
    assert_eq!(providers[0]["provider"], "graphify", "providers must be sorted");
    assert_eq!(providers[1]["provider"], "semantica");
    let semantica_status = &providers[1]["status"];
    assert!(
        semantica_status == "absent" || semantica_status == "ok",
        "unexpected status: {semantica_status}"
    );
}
```

- [ ] **Step 5: Run tests to verify they pass**

Run: `cargo test --lib provider && cargo test --test doctor`
Expected: PASS, four tests total.

- [ ] **Step 6: Run the toolchain gates**

Run: `cargo fmt --all -- --check && cargo clippy --all-targets --all-features -- -D warnings && cargo test --all-features`

- [ ] **Step 7: Commit**

```bash
git add src/ tests/
git commit
```

Title: `✨ report provider preconditions by delegation: doctor`

---

### Task 9: Verify the success criteria against `agent-sandbox`

**Files:**
- Create: `docs/superpowers/plans/2026-09-06-grafite-mvp-verification.md`
- Modify: none

This task produces evidence, not code. Every claim below must be supported by
pasted command output; `AGENTS.md` §4.4 forbids asserting success without it.

- [ ] **Step 1: Build the release binary**

Run: `cargo build --release`
Record the resulting path: `target/release/grafite`.

- [ ] **Step 2: Verify criteria 1 and 2 — completeness and determinism**

```bash
cd /home/v/Data/Projects/agent-sandbox
/home/v/Data/Projects/Grafite/target/release/grafite sync > /tmp/grafite-sync-1.json
cp .grafite/state/records/decisions.jsonl /tmp/records-1.jsonl
/home/v/Data/Projects/Grafite/target/release/grafite sync > /tmp/grafite-sync-2.json
diff /tmp/records-1.jsonl .grafite/state/records/decisions.jsonl && echo "DETERMINISTIC"
git log --oneline | wc -l
git log --format='%s' | grep -c '🕸️ sync knowledge graph'
```

Expected: `diff` reports no differences. The record count in
`/tmp/grafite-sync-1.json` equals total commits minus graph-sync commits.
Record all four numbers in the verification document.

- [ ] **Step 3: Verify criterion 3 — `why` on a real file**

```bash
/home/v/Data/Projects/Grafite/target/release/grafite why cli/asb/doctor.py | python3 -m json.tool
```

Expected: exit 0, valid JSON, at least one record carrying a non-empty
`rationale`. Paste the output.

- [ ] **Step 4: Verify criterion 4 — Semantica absent is healthy**

```bash
/home/v/Data/Projects/Grafite/target/release/grafite doctor | python3 -m json.tool
```

Expected: `"healthy": true` with `semantica` reported `absent`. Paste it.

- [ ] **Step 5: Verify criterion 5 — no excluded path leaked**

The criterion is about **edge targets**, so the check must read `.edges[].to`
and nothing else. A line-wide `grep` over the JSONL would also match the
`rationale` prose, where a commit author may legitimately mention `.env` or
`id_ed25519` in their own message — exactly the ingestion the design document
declares and accepts in §8. Matching there is expected, not a leak.

```bash
python3 - <<'PY'
import json, re
pattern = re.compile(r'\.env|\.pass\b|id_ed25519|agent-sandbox\.toml')
records = "/home/v/Data/Projects/agent-sandbox/.grafite/state/records/decisions.jsonl"
leaks = [
    (json.loads(line)["id"], edge["to"])
    for line in open(records)
    for edge in json.loads(line).get("edges", [])
    if pattern.search(edge["to"])
]
print(f"edge-target leaks: {len(leaks)}")
for leak in leaks:
    print(" ", leak)
PY
```

Expected: `edge-target leaks: 0`. Any leak is a release blocker; stop and
report it.

- [ ] **Step 6: Verify criterion 1 — no network access**

```bash
cd /home/v/Data/Projects/agent-sandbox
rm -rf .grafite/state
unshare -r -n /home/v/Data/Projects/Grafite/target/release/grafite sync
```

Expected: exit 0 with no network namespace. If `unshare` is unavailable on
this host, record that fact and verify by inspection instead: the crate has
three dependencies, none of which opens a socket.

- [ ] **Step 7: Confirm the fixture repository left no residue**

```bash
cd /home/v/Data/Projects/agent-sandbox && git status --short
```

Expected: `.grafite/` must not appear as untracked noise once it is added to
that repository's `.gitignore`. If it does appear, add
`/.grafite/state/` to `agent-sandbox`'s `.gitignore` as part of this task and
note it as a change made to a second repository, requiring its own
authorization.

- [ ] **Step 8: Write the verification document**

Create `docs/superpowers/plans/2026-09-06-grafite-mvp-verification.md`
recording, for each of the seven success criteria in the spec: the command
run, its verbatim output, and pass or fail. Criterion 6 (a body without
headings yields an empty rationale) and criterion 7 (toolchain gates) are
covered by the automated suite — cite the passing test names and the gate
output rather than re-running them by hand.

- [ ] **Step 9: Commit**

```bash
git add docs/superpowers/plans/2026-09-06-grafite-mvp-verification.md
git commit
```

Title: `📝 record mvp verification evidence: verification`

---

## Self-Review

**1. Spec coverage**

| Spec section | Task |
| :--- | :--- |
| §1 Problem, §2 Responsibilities | Context; no code. |
| §3 Architecture — records under `.grafite/state/` | Task 6 |
| §3 Architecture — `.grafite/config.toml` | **Deviation, declared in Global Constraints.** Deferred; needs a fourth crate. |
| §4 Scope — three commands only | Tasks 1, 6, 7, 8 |
| §5 CLI — stream discipline, exit codes | Tasks 1, 7 |
| §5 CLI — `doctor` delegates to the provider | Task 8 |
| §6 Record shape, full SHA, `short_id` | Tasks 3, 6 |
| §6 `TOUCHES` edge, target classification | Tasks 5, 6 |
| §6 Rule 1 — headings to rationale | Task 4 |
| §6 Rule 2 — discard graph-sync commits | Tasks 4, 6 |
| §6 Rule 3 — never read file contents | Tasks 4, 5 (parsing is over commit text and path strings only) |
| §7 Semantica seam | Deliberately no code. Task 8 states why. |
| §8 Security — exclusion policy | Task 5 |
| §8 Security — no network | Task 9, Step 6 |
| §9 Rust structure | File Structure table |
| §10 Criteria 1–5 | Task 9 |
| §10 Criterion 6 | Task 4, `body_without_headings_yields_empty_rationale_not_an_error` |
| §10 Criterion 7 | Gate step in every task |

`src/main.rs` in the spec's structure carries argument parsing; `src/commands.rs`
was added to keep orchestration out of `main.rs`. This is an addition to the
spec's file list, not a contradiction of it.

**2. Placeholder scan**

No `TBD`, no "add error handling", no "similar to Task N". Every code step
carries the actual code. Task 9 is evidence-gathering by design and names the
exact commands and expected outputs.

**3. Type consistency**

- `Target::id()` is defined in Task 3 and used in Tasks 5, 6, 7 under that name.
- `Edge.edge_type` is serialized as `type`; tests in Tasks 3 and 6 assert the
  wire name `"TOUCHES"`, not the Rust name.
- `records_path` is defined in Task 6 and reused in Task 7.
- `Report::is_acceptable` is defined and asserted in Task 8 only.
- `record::SCHEMA_VERSION` is imported into `src/commands.rs` in Task 6 and used
  again by `why` and `doctor` in Tasks 7 and 8.
- `support::fixture_repo` and `support::commit_file` are defined in Task 2 and
  used unchanged in Tasks 6, 7, and 8.
- Task 2 introduces `src/lib.rs` and converts `src/main.rs` from `mod error;`
  to `use grafite::error;`. Every later task assumes the library layout.
