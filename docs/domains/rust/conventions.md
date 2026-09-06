# Rust Conventions

Canonical rules for authoring and reviewing Rust code in this repository.

## 1. Toolchain Gates (Empirical Verification)

No task is complete until these pass on the changed code:

```bash
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-features
```

1. `rustfmt` defaults are authoritative; do not hand-format around them or add per-file overrides.
2. Clippy warnings are errors. Silence a lint only with a scoped `#[allow(...)]` **plus** a comment stating why; never a crate-wide blanket allow.
3. Report failures verbatim. Never assert that a gate passed without having run it.

## 2. Error Handling & Panic Policy

1. Fallible operations return `Result`. Errors propagate with `?`; they are not swallowed, logged-and-ignored, or converted into sentinel values.
2. **No `unwrap()`, `expect()`, `panic!()`, or indexing that can panic in non-test code**, except where an invariant is locally proven and stated in a comment.
3. Errors carry actionable context: what was attempted, on which input, and why it failed. An error an agent cannot act on is a defect.
4. `unsafe` requires explicit human authorization and a `// SAFETY:` comment proving every invariant it relies on.

## 3. Dependency Discipline

1. Every new dependency is a decision presented to the human engineer before it is added, with the alternative of writing it inline stated.
2. Prefer the standard library and existing dependencies over adding a crate for a small amount of code.
3. `Cargo.lock` is committed (binary crate). Do not upgrade dependencies as a side effect of an unrelated task.

## 4. Testing

1. New behavior ships with a test that fails without the change.
2. Unit tests live next to the code in `#[cfg(test)] mod tests`; end-to-end CLI behavior is tested through the binary's real interface, not by calling internals.
3. Tests assert on observable behavior — exit codes, stdout payloads, produced files — not on internal call sequences.

## 5. Surgical Scope

Refactors, renames, and formatting sweeps that are not required by the current task belong in a separate, explicitly authorized change.
