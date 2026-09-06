# Grafite MVP — Provenance Records from Verifiable Git Facts

> Design specification. Status: approved for planning. Date: 2026-09-06.

---

## 1. Problem the MVP Solves

An agent working in `agent-sandbox` can already ask what the codebase *is*:
Graphify answers architecture, dependency, and relationship questions from a
persistent knowledge graph.

No tool answers what the codebase *records about its own changes*. When an
agent encounters `cli/asb/doctor.py`, it can read the code and the graph, but
it cannot recover the reasoning that accompanied each change to that file —
reasoning that already exists, written by hand, in the structured commit
bodies mandated by `docs/domains/git/commit-conventions.md`, and in the
specifications under `docs/superpowers/`.

That reasoning is high-quality and versioned, and it is effectively
unreadable at scale: recovering it today means `git log`-ing by hand and
reading 88 commit bodies.

**The MVP makes that corpus queryable from verifiable Git facts alone.**

### Explicit non-goal

The MVP does not model *decisions*. A commit is evidence that something
changed, plus the rationale its author declared. It is not the atomic unit of
a decision: one commit may carry several decisions, none at all, or a mix of
fixes, validations, and operational changes. Retrospective ingestion cannot
recover a granularity the history never expressed, and inventing one would
make every downstream causal claim unfalsifiable.

## 2. Responsibilities

| Component | Owns | Does not own |
| :--- | :--- | :--- |
| **Graphify** | Structural knowledge: code AST, dependencies, documents, communities. Answers "what is this and what connects to it". | Change history, authorial rationale. |
| **Grafite** | Coordination, and the canonical provenance record. Extracts verifiable facts from Git and answers questions over them. | Structural analysis; it never re-implements Graphify. |
| **Semantica** | *Deferred.* Future causal and ontological reasoning over explicit decisions. | Anything in this MVP. It is not a source of truth at any stage. |

Human knowledge under `docs/domains/` remains the source of truth for
conventions. Grafite reads Git; it never rewrites documentation, and no
derived artifact supersedes a versioned human document.

## 3. Minimal Architecture

```text
Git history ─┐
specs/plans ─┼─→ Grafite Core (Rust) ─→ canonical provenance records ─→ native query
ADRs (§6) ───┘                             (.grafite/state/records/)
```

Deferred, and specified in §7 so that it stays possible:

```text
canonical provenance records ─→ Semantica adapter (Python) ─→ Semantica reasoning
```

State locality:

| Path | Versioned | Contents |
| :--- | :--- | :--- |
| `.grafite/config.toml` | Yes | Document source globs, and the provider enablement flag reserved by §7. Optional throughout the MVP: absent means built-in defaults. |
| `.grafite/state/**` | No (gitignored) | Derived records. Rebuildable from Git at any time. |

Derived state is local and disposable because extraction is deterministic:
any clone regenerates identical records from the same history. This avoids a
merge driver, keeps diffs clean, and reduces the surface on which derived
content could leak back into the repository.

## 4. Scope and Non-Scope

**In scope**

- Extraction of provenance records from commits, specs, plans, and ADRs.
- Exactly one edge type, derived from `git show --name-only`.
- Three commands: `sync`, `doctor`, `why`.
- Provider detection and reporting for Graphify and Semantica.

**Out of scope**

- `kind: decision`, and the `CAUSED` / `INFLUENCED` / `PRECEDENT_FOR` edges.
- `grafite trace`. It returns when causal semantics exist to honor it.
- The Semantica adapter implementation (§7 fixes the seam only).
- `grafite init`. The MVP is zero-config; enabling a provider is one
  hand-written line, which does not justify a wizard.
- Natural-language query routing, daemon, TUI, dashboard, Serena.
- Any provider abstraction generalized beyond the two providers that exist.

## 5. CLI

Contract rules in `docs/domains/rust/cli-contract.md` are binding.

```bash
grafite sync            # extract provenance records from Git history
grafite doctor          # report provider preconditions
grafite why <path>      # provenance records that touched <path>, with rationale
```

- `sync` is idempotent and offline. It reads Git and writes
  `.grafite/state/records/`. It never mutates the repository or provider state.
- `doctor` unifies the health check across providers. For Graphify it
  **delegates to the provider's own verification** — `.graphify/setup.sh
  --verify-only` where present, otherwise the provider's own version query —
  and normalizes the result into Grafite's payload. Grafite does not
  re-implement the pin, hook, and merge-driver checks; duplicating them would
  create a second definition of Graphify health that silently drifts from the
  first. For Semantica it reports presence only. **Semantica absent is `ok`, not a
  warning**: absence is the normal state. No MVP command requires Graphify, so
  a Graphify finding is reported, never fatal.
- `why` answers natively from the records. It reports the records that touched
  the path and their declared rationale. It asserts no causality.

`stdout` carries only the payload; diagnostics go to `stderr`; collections are
sorted; paths are repository-relative.

## 6. Ingestion Strategy

### Record shape

One JSON object per line, `schema_version` explicit, keys and arrays sorted.

```json
{"schema_version":1,
 "id":"commit:4bc618c55e888ceb468e06fe89597c42ff052751","short_id":"4bc618c5","kind":"commit",
 "gitmoji":"✨","subject":"diagnose secret service in doctor and validate login lifecycle","scope":"cli",
 "authored_at":"2026-09-06T14:21:03+00:00",
 "rationale":{"new_features":["Added diagnostic check in cli/asb/doctor.py for the Secret Service singleton (asb-keyring)."],
              "architecture":["Replaced custom polling loop with the reusable, non-mutating check_keyring_service() health check."],
              "validations":["Added unit tests in tests/unit/test_doctor.py covering all four failure cases."],
              "security":[],"outcome":[]},
 "edges":[{"type":"TOUCHES","to":"file:cli/asb/doctor.py"}]}
```

`kind` is one of:

| `kind` | Identity | Role |
| :--- | :--- | :--- |
| `commit` | **Full 40-character SHA** | Change event, carrying the rationale its author declared. A `short_id` is carried alongside for human-facing output only; it is never the canonical identity, never a key, and never the target of an edge. |
| `spec`, `plan` | Repository-relative path | Provenance document. |
| `adr` | Repository-relative path | Reserved. No source is configured by default, because the repository has no ADR directory today. |

`file` is never a record. It appears only as an edge target: an implementation
artifact identified by its path.

Document sources are path globs, defaulting to `docs/superpowers/specs/*.md`
and `docs/superpowers/plans/*.md`, and overridable in `.grafite/config.toml`.

A document record carries its `kind` and its path, and nothing else. It
deliberately does not carry a title or summary, because deriving either would
require opening the file and would break the invariant in §8.

### Edges

`TOUCHES` is the only edge type in the MVP, derived from
`git show --name-only`. It states a fact Git guarantees.

The edge target is classified deterministically by matching the path against
the configured globs, in this order:

| Match | Target identity |
| :--- | :--- |
| Specs glob | `spec:<path>` |
| Plans glob | `plan:<path>` |
| ADR glob, when configured | `adr:<path>` |
| Anything else | `file:<path>` |

A target identity therefore agrees with the record identity of the same
document, which is what makes the traversal
`file:… ← TOUCHES ← commit → TOUCHES → spec:…` resolvable rather than merely
described. Classification is a path-glob match and reads no file content.

The traversal yields an association and stays one. Grafite does not upgrade it
to causality.

### Rules

1. `rationale` is populated by splitting the commit body on the five headings
   fixed by `commit-conventions.md`. A body without headings yields an empty
   rationale, not a parse failure.
2. Commits titled `🕸️ sync knowledge graph` carry no body by design and are
   discarded.
3. Extraction reads commit *messages* and file *paths*. It never reads the
   content of the files a commit touched.

### Known limitation, stated deliberately

A commit that implements a specification without touching that
specification's file produces no edge to it. The date-slug naming convention
would allow a fuzzy match; the MVP rejects it, because a heuristic link is
indistinguishable in output from a verified one. Closing that gap requires an
explicit marker written at commit time, which is prospective work.

## 7. Semantica Seam — Deferred, Not Abandoned

No adapter ships in the MVP. Empirical finding from the spike: Semantica's
causal analysis traverses `CAUSED`, `INFLUENCED`, and `PRECEDENT_FOR`. A graph
containing only `TOUCHES` gives it nothing to reason about. Building the
adapter now would prove an integration without delivering a capability.

The seam is specified so that it stays cheap to close:

1. **Opt-in only.** Grafite never installs Semantica, and never invokes it
   unless `.grafite/config.toml` enables it.
2. **Process boundary.** One JSON object in on `stdin`, one out on `stdout`,
   `schema_version` on both. The Rust core depends on that protocol and on
   nothing else.
3. **No coupling to internals.** The core never references a Semantica class,
   module, or config format. Every such reference lives in the adapter.
4. **Output containment.** The spike measured Semantica writing logs, Python
   tracebacks, and decorative output to `stdout` while leaving `stderr` empty,
   under `--json`. The adapter must therefore capture and discard Semantica's
   `stdout` and emit only the Grafite contract. Provider errors are reported on
   `stderr` with the provider's identity preserved, per `cli-contract.md §5.1`.
5. **Project-scoped state.** The adapter passes explicit paths and never reads
   or creates `~/.semantica`. Its namespace is `.grafite/state/semantica/`.
6. **Pinned.** Semantica is pre-1.0 (0.6.8 measured); any future adapter pins
   an exact version, as `.graphify/setup.sh` pins `graphifyy==0.9.51`.

The adapter is implemented when `kind: decision` and real causal edges exist.

### Recorded limitation

Semantica writes a random UUID `graph_id` on every save. This does not matter
while its state is local and disposable. It is the reason promoting
`.grafite/state/` to versioned would not be free.

## 8. Security Model

The central invariant is structural rather than enumerative:

> **Grafite never reads the content of repository files.**

It consumes commit messages and path names only. Secrets living in file
*contents* — `.env` values, keys, tokens in source — are unreachable by
construction, not by exclusion list.

That invariant must not be read as "no ingestion surface at all". Commit
message bodies are ingested verbatim into `rationale`, and a commit body can
contain pasted diagnostic output, a credential, or a token. This is the same
exposure Graphify already accepts by indexing the repository, and the corpus
is one humans wrote and reviewed. The mitigation is the locality decision
already made rather than a new scanner: records are local and gitignored, so
nothing derived re-enters the repository or reaches a network.

What remains, and how it is handled:

1. **Path names leak too** (`secrets/prod-db-password.txt`). Grafite applies
   the exclusion policy already encoded in `.graphify/project.py` — `.env*`,
   `*.pass`, `id_ed25519*`, `.agent-sandbox.toml`, `state/`, `scratch/` — to
   decide which paths may become edge targets. One policy, two consumers: this
   is how exclusion consistency across providers is achieved concretely.
2. **No network.** Neither the core nor any future adapter performs network
   access, and no API key is read. A future adapter constructs its graph with
   embeddings, Node2Vec, and community detection disabled.
3. **No global state.** Nothing reads or writes `~/.semantica` or any other
   user-global provider configuration.
4. **Read-only toward the repository.** `sync` writes only under
   `.grafite/state/`. No command mutates repository or provider state as a
   side effect.

## 9. Minimal Rust Structure

```text
src/main.rs          argument parsing, exit codes, stream discipline
src/git.rs           commit and path reading via the git subprocess
src/record.rs        canonical record types and JSONL serialization
src/extract.rs       commit body to record
src/query.rs         native `why` over records
src/provider/mod.rs  provider detection and the adapter protocol definition
```

Nothing in `src/` outside `provider/` refers to a provider by name. That is
the property that keeps extraction to a standalone repository mechanical.

**Dependencies submitted for approval** per `conventions.md §3`: `clap`,
`serde`, `serde_json`. Git access uses the `git` subprocess rather than
`libgit2`, avoiding a heavy native dependency; `.graphify/project.py` already
sets that precedent by shelling out to `git ls-files`.

## 10. Success Criteria

Verified against the real history of `agent-sandbox`, which holds 88 commits
at the time of writing.

1. `grafite sync` processes every commit reachable from `HEAD`, discards
   graph-sync commits, exits 0, and performs no network access.
2. Two runs at the same `HEAD` produce byte-identical output.
3. `grafite why cli/asb/doctor.py` returns the records that touched it with
   their rationale, in the documented shape, exit 0.
4. `grafite doctor` reports Semantica as absent with status `ok`, and `sync`
   and `why` function normally in that state.
5. No excluded path (`.env*`, `*.pass`, `id_ed25519*`) appears in any record —
   asserted by an automated test.
6. A commit body lacking the mandated headings yields an empty rationale and a
   successful run, not a failure.
7. `cargo fmt --all -- --check`, `cargo clippy --all-targets --all-features --
   -D warnings`, and `cargo test --all-features` pass.
