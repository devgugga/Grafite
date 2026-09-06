@AGENTS.md

# Claude Code Configuration

Project-wide governance, engineering guidelines, and agent workflow standards are defined in `AGENTS.md` (imported above).

## Available Subagents (`.claude/agents/`)

* [`commit-curator`](.claude/agents/git/commit-curator.md): Normalizes and creates standardized local commits with Gitmoji and mandatory structured body.
* [`rust-reviewer`](.claude/agents/rust/rust-reviewer.md): Read-only reviewer for Rust correctness and the CLI output contract. Use proactively after writing or modifying Rust code, before human gates.

## Skills (`.claude/skills/`)

The directory `.claude/skills/` is the **canonical source of truth** for all skills in this repository. After adding or modifying a skill, run:
```bash
node scripts/sync-skills.mjs
```

## Domain Packs (`docs/domains/`)

Agent behavior is defined by the domain packs, never by the wrapper prompts. To change how an agent behaves, edit the canonical document:
* [`docs/domains/rust/`](docs/domains/rust/README.md): Rust and CLI conventions.
* [`docs/domains/git/`](docs/domains/git/README.md): commit and branch governance.
* [`docs/domains/agent-authoring/`](docs/domains/agent-authoring/README.md): how to create or modify agents and skills.
