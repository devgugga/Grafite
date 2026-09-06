# CLI Contract

Grafite's primary consumer is an autonomous agent. The command-line surface is therefore an **API**, and the rules below are correctness requirements, not ergonomics preferences.

## 1. Stream Separation

1. `stdout` carries **only** the command's result payload — the data a caller parses.
2. `stderr` carries diagnostics: progress, warnings, errors, and human-readable explanation.
3. Never interleave logs into `stdout`. A caller that pipes `stdout` into a parser must never receive a log line.

## 2. Exit Codes

1. `0` means the command achieved what it was asked to do. Any other outcome is non-zero.
2. Partial success is a failure unless the command explicitly documents partial semantics and reports what succeeded and what did not.
3. Exit codes are stable and documented per command; do not repurpose an existing code.

## 3. Machine-Readable Output

1. Structured output must be parseable without heuristics — no banners, spinners, or decorative framing inside `stdout`.
2. When a command emits structured data, its shape is documented and additive: fields may be added, never renamed or removed without an explicit interface change.
3. Human-oriented formatting (color, tables, progress) must be suppressed automatically when `stdout` is not a TTY, and overridable by an explicit flag.
4. Errors reported to an agent state the failing operation, the offending input, and the remediation — in that order.

## 4. Determinism

1. The same inputs produce the same output; ordering of collections in output is explicitly sorted, never dependent on hash iteration order.
2. Timestamps, absolute paths, and machine-specific values do not appear in output unless the command's purpose is to report them.

## 5. Underlying Tools

Grafite unifies external engineering-intelligence tools. Therefore:
1. A failure of an underlying tool is surfaced with its identity and its own error output; it is never rewritten into a generic message or silently swallowed.
2. A missing or incompatible underlying tool is reported as an actionable precondition failure, not as a crash.
3. Grafite never mutates a user's repository or external tool state as an implicit side effect of a read-only command.
