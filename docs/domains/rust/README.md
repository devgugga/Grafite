# Rust Domain Pack

Single Source of Truth (SSoT) for how Grafite is written in Rust and how its CLI behaves toward its primary consumer: AI agents.

## Documents

* [`conventions.md`](./conventions.md): Toolchain gates, error handling, panic policy, dependency discipline, and `unsafe` governance.
* [`cli-contract.md`](./cli-contract.md): Stream separation, exit codes, machine-readable output, and interface stability.

## Scope Discipline

This pack starts **deliberately minimal**. It records only rules that are already decided or that hold regardless of pending architecture choices (module layout, async runtime, CLI parser, subprocess strategy, plugin model).

When such a decision is made, document it here **first**, then let the wrappers inherit it. Never encode a speculative convention: an unmade decision written down as a rule is worse than no rule, because agents will enforce it.
