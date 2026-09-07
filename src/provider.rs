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
    match Command::new(&script)
        .arg("--verify-only")
        .current_dir(repo)
        .output()
    {
        Ok(output) if output.status.success() => Report {
            provider: "graphify".to_string(),
            status: Status::Ok,
            detail: String::from_utf8_lossy(&output.stdout).trim().to_string(),
        },
        Ok(output) => {
            let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
            let detail = if stderr.is_empty() {
                String::from_utf8_lossy(&output.stdout).trim().to_string()
            } else {
                stderr
            };
            Report {
                provider: "graphify".to_string(),
                status: Status::Failed,
                detail,
            }
        }
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
