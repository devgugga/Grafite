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
    // Error::new is the crate error constructor specified in Task 1 interfaces,
    // consumed across domain modules in subsequent tasks.
    #[allow(dead_code)]
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_error_with_required_fields() {
        let err = Error::new("read", "foo.txt", "file not found", "check path");
        assert_eq!(
            err.to_string(),
            "failed to read: input foo.txt: file not found: check path"
        );
    }
}
