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
        for path in [
            "cli/asb/doctor.py",
            "docs/domains/rust/conventions.md",
            "AGENTS.md",
        ] {
            assert!(!is_excluded(path), "must not exclude {path}");
        }
    }
}
