# Domain Packs

Domain knowledge in this repository is stored as portable, tool-agnostic Markdown packs consumed by all AI tools (Google Antigravity, Claude Code, OpenAI Codex, Gemini CLI).

## Available Domain Packs

- [`rust/`](./rust/README.md): Rust toolchain gates, error handling, dependency discipline, and the CLI output contract for agent consumers.
- [`git/`](./git/commit-conventions.md): Git commit conventions, Gitmoji standards, branch policy, and structured commit bodies.
- [`agent-authoring/`](./agent-authoring/README.md): Conventions, platform layouts, model routing, and validation for authoring agents and skills.

## Adding a New Domain Pack

1. Create `docs/domains/<domain>/` with a `README.md` and the canonical documents.
2. Create the thin wrappers per [`agent-authoring/platform-layouts.md`](./agent-authoring/platform-layouts.md).
3. Register the subagent in `AGENTS.md`, `CLAUDE.md`, and `GEMINI.md`.

## Golden Rule

> Agent wrappers point to the domain documents. Domain knowledge is never copied into a wrapper or skill prompt. If you find yourself writing domain rules inside an agent configuration file, move them into the corresponding Domain Pack.
