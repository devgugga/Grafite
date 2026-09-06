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
