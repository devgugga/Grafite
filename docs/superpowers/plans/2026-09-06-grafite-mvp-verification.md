# Grafite MVP Verification Report

Date: 2026-09-06  
Tested against repository: `/home/v/Data/Projects/agent-sandbox`  
Grafite Binary: `/home/v/Data/Projects/Grafite/target/release/grafite` (version `0.1.0`)  
Spec Reference: `docs/superpowers/specs/2026-09-06-grafite-mvp-design.md` §10  
Plan Reference: `docs/superpowers/plans/2026-09-06-grafite-mvp.md` (Task 9)

---

## Executive Summary

| # | Success Criterion | Status | Empirical Evidence Summary |
| :- | :--- | :--- | :--- |
| **1** | Completeness & no network access | **PASS** | 88 commits read, 3 graph-sync discarded, 85 records created. Ran in isolated network namespace (`unshare -r -n`) with exit code 0. |
| **2** | Determinism | **PASS** | Two consecutive `sync` runs produced byte-identical JSONL (`diff` reported zero differences). |
| **3** | `why <path>` on real file | **PASS** | `why cli/asb/doctor.py` returned 7 matching provenance records sorted by ID with rich rationale, valid JSON, exit code 0. |
| **4** | `doctor` reports absent Semantica as healthy | **PASS** | `doctor` output reported `"healthy": true` with `semantica` status `"absent"`; `sync` and `why` functional. |
| **5** | Excluded paths not leaked | **PASS** | Automated test `sync_omits_excluded_paths` passes; zero excluded paths in `.edges[].to` across all 85 records in `agent-sandbox`. |
| **6** | Heading-less commit bodies yield empty rationale | **PASS** | Automated unit test `extract::tests::body_without_headings_yields_empty_rationale_not_an_error` passes. |
| **7** | Toolchain gates pass | **PASS** | `cargo fmt --all -- --check`, `cargo clippy --all-targets --all-features -- -D warnings`, and 33/33 tests pass in `cargo test --all-features`. |

---

## Criterion 1: Ingestion Completeness & No Network Access

> *`grafite sync` processes every commit reachable from `HEAD`, discards graph-sync commits, exits 0, and performs no network access.*

### 1.1 Commit Processing & Graph-Sync Filtering

**Commands:**
```bash
cd /home/v/Data/Projects/agent-sandbox
/home/v/Data/Projects/Grafite/target/release/grafite sync > /tmp/grafite-sync-1.json
git log --oneline | wc -l
git log --format='%s' | grep -c '🕸️ sync knowledge graph'
cat /tmp/grafite-sync-1.json
```

**Verbatim Output:**
```text
$ git log --oneline | wc -l
88

$ git log --format='%s' | grep -c '🕸️ sync knowledge graph'
3

$ cat /tmp/grafite-sync-1.json
{"commits_read":88,"records":85,"schema_version":1}
```

**Verification Analysis:**
- Total commits reachable from `HEAD`: `88`
- Graph-sync commits matching `🕸️ sync knowledge graph`: `3`
- Commits read: `88`
- Records produced: `85` (`88 - 3 = 85`)
- Exit code: `0`

### 1.2 No Network Access

**Command:**
```bash
cd /home/v/Data/Projects/agent-sandbox
rm -rf .grafite/state
unshare -r -n /home/v/Data/Projects/Grafite/target/release/grafite sync
```

**Verbatim Output:**
```json
{"commits_read":88,"records":85,"schema_version":1}
```

**Verification Analysis:**
- `unshare -r -n` unshares the user and network namespaces, creating an isolated network environment with no external network interfaces and unconfigured loopback.
- `grafite sync` completed with exit code 0.
- Dependency verification in `Cargo.toml`:
  ```toml
  [dependencies]
  clap = { version = "4", features = ["derive"] }
  serde = { version = "1", features = ["derive"] }
  serde_json = "1"
  ```
  Zero network crates are included in the dependency tree.

**Result:** **PASS**

---

## Criterion 2: Determinism Across Runs

> *Two runs at the same `HEAD` produce byte-identical output.*

### Verification

**Commands:**
```bash
cd /home/v/Data/Projects/agent-sandbox
/home/v/Data/Projects/Grafite/target/release/grafite sync > /tmp/grafite-sync-1.json
cp .grafite/state/records/decisions.jsonl /tmp/records-1.jsonl
/home/v/Data/Projects/Grafite/target/release/grafite sync > /tmp/grafite-sync-2.json
diff /tmp/records-1.jsonl .grafite/state/records/decisions.jsonl && echo "DETERMINISTIC"
```

**Verbatim Output:**
```text
DETERMINISTIC
```

**Verification Analysis:**
- The diff command exited with code 0 and produced no differences between runs.
- `grafite-sync-1.json` and `grafite-sync-2.json` both contain `{"commits_read":88,"records":85,"schema_version":1}`.
- Sorting of commits by commit ID in descending order and edges in alphabetical order guarantees deterministic, byte-identical JSONL output across multiple runs at the same `HEAD`.

**Result:** **PASS**

---

## Criterion 3: `why` on Real File

> *`grafite why cli/asb/doctor.py` returns the records that touched it with their rationale, in the documented shape, exit 0.*

### Verification

**Command:**
```bash
cd /home/v/Data/Projects/agent-sandbox
/home/v/Data/Projects/Grafite/target/release/grafite why cli/asb/doctor.py | python3 -m json.tool
```

**Verbatim Output:**
```json
{
    "path": "cli/asb/doctor.py",
    "records": [
        {
            "authored_at": "2026-09-05T15:35:39-03:00",
            "edges": [
                {
                    "to": "file:cli/asb/doctor.py",
                    "type": "TOUCHES"
                },
                {
                    "to": "file:cli/asb/install.py",
                    "type": "TOUCHES"
                },
                {
                    "to": "file:docs/domains/sandbox/README.md",
                    "type": "TOUCHES"
                },
                {
                    "to": "file:tests/test-guard.sh",
                    "type": "TOUCHES"
                },
                {
                    "to": "file:tests/unit/test_doctor.py",
                    "type": "TOUCHES"
                },
                {
                    "to": "file:tests/unit/test_install.py",
                    "type": "TOUCHES"
                }
            ],
            "gitmoji": "\u2728",
            "id": "commit:05407c65eb00819ced24419b1368de3221fd5fe8",
            "kind": "commit",
            "rationale": {
                "architecture": [
                    "Extract `_link()` helper in `install.py` to eliminate duplicated symlink creation logic. This ensures all four links (three guards + CLI) follow the same pattern: symlinks (never copies), cleaned up before replacement."
                ],
                "new_features": [
                    "`asb-agent install-guards` now installs `~/.local/bin/asb-agent` symlink pointing to the checkout's `cli/asb-agent` entrypoint, restoring the v1 capability of running the CLI from any directory.",
                    "`asb-agent doctor` adds new check \"asb-agent aponta para este checkout\" to detect silent failures when the checkout moves: without this symlink, the CLI only runs from within the checkout, and the absence is invisible to the operator."
                ],
                "outcome": [
                    "The operator can now run `asb-agent` from any directory, not only from within the checkout. All symlinks are validated by `doctor`, which exits with code 1 if any link is stale or missing. Moving the checkout directory breaks the links, and `doctor` catches this immediately instead of failing silently. Test suite passes: 117 unit tests OK; test-guard.sh 13/13; test-doctor.sh 12/12; test-image.sh 27/27; `asb-agent doctor` from `/tmp` returns 0."
                ],
                "security": [],
                "validations": [
                    "Add two new unit tests in `test_install.py`:",
                    "`test_guards_instala_o_proprio_asb_agent_no_path`: validates that the CLI link is created correctly.",
                    "`test_guards_substitui_asb_agent_de_checkout_antigo`: validates that old stale links pointing to moved checkouts are replaced.",
                    "Add one new unit test in `test_doctor.py`:",
                    "`test_doctor_acusa_asb_agent_fora_do_path`: validates that doctor exits with code 1 when the CLI link is missing.",
                    "Fix two broken assertions in `tests/test-guard.sh`:",
                    "Update hardcoded image name from `agent-sandbox-base` (stale) to `agent-sandbox:latest` with explicit tag.",
                    "Add `--pull=never` flag to prevent podman searching Docker Hub when local image is missing (policy enforcement).",
                    "Add positive control check `require` that aborts immediately with clear message if image does not exist locally."
                ]
            },
            "schema_version": 1,
            "scope": "install and doctor",
            "short_id": "05407c65",
            "subject": "restore asb-agent to PATH and add validation"
        },
        {
            "authored_at": "2026-09-05T10:29:02-03:00",
            "edges": [
                {
                    "to": "file:cli/asb/doctor.py",
                    "type": "TOUCHES"
                },
                {
                    "to": "file:cli/asb/lifecycle.py",
                    "type": "TOUCHES"
                },
                {
                    "to": "file:image/entrypoint.sh",
                    "type": "TOUCHES"
                },
                {
                    "to": "file:tests/test-doctor.sh",
                    "type": "TOUCHES"
                },
                {
                    "to": "file:tests/test-toolcache.sh",
                    "type": "TOUCHES"
                }
            ],
            "gitmoji": "\u2728",
            "id": "commit:1fc2d74aa8baa9d175c43c363e9cd95880bb0858",
            "kind": "commit",
            "rationale": {
                "architecture": [
                    "Decouple workspace ephemeral lifecycles from heavy tool installations and build caches. Destroying or recreating workspaces preserves installed toolchains and downloaded package caches across development sessions, reducing cold-boot times and network overhead. Ensure toolcache volume provisioning is fully idempotent via ensure_toolcache_volume in cli/asb/lifecycle.py before starting agent containers."
                ],
                "new_features": [
                    "Create and mount the persistent `asb-toolcache` volume into agent containers at /run/asb-toolcache, sharing tool binaries and caches across workspace lifecycles. Materialize symlinks at container boot in image/entrypoint.sh pointing ~/.local/share/mise, ~/.cache, and ~/.m2 to the shared toolcache volume with correct user ownership and directory structures. Automatically detect root or nested mise.toml configuration files on `asb-agent up` and execute `mise install` non-fatally, logging status cleanly without blocking container startup on install errors. Add `asb-toolcache` volume health check to `asb-agent doctor` with actionable repair suggestion `podman volume create asb-toolcache`."
                ],
                "outcome": [
                    "Workspaces share mise runtimes and tool caches out of the box without re-downloading compilers or dependencies across workspace recycles."
                ],
                "security": [],
                "validations": [
                    "Add integration test suite tests/test-toolcache.sh verifying volume persistence across workspace teardown (`down`), cross-workspace cache sharing (`up`), symlink integrity, and automated mise invocation. Expand tests/test-doctor.sh to assert `asb-toolcache` volume presence checks. Rebuild base image and verify 94/94 unit tests and full regression suite (auth, nested, services, network, transaction) pass."
                ]
            },
            "schema_version": 1,
            "scope": "asb-toolcache and mise",
            "short_id": "1fc2d74a",
            "subject": "share tools and cache across workspaces"
        },
        {
            "authored_at": "2026-09-05T00:13:26-03:00",
            "edges": [
                {
                    "to": "file:cli/agent-sandbox",
                    "type": "TOUCHES"
                },
                {
                    "to": "file:cli/asb-agent",
                    "type": "TOUCHES"
                },
                {
                    "to": "file:cli/asb-guard",
                    "type": "TOUCHES"
                },
                {
                    "to": "file:cli/asb/doctor.py",
                    "type": "TOUCHES"
                },
                {
                    "to": "file:cli/asb/install.py",
                    "type": "TOUCHES"
                },
                {
                    "to": "file:cli/asb/lifecycle.py",
                    "type": "TOUCHES"
                },
                {
                    "to": "file:cli/asb/podman.py",
                    "type": "TOUCHES"
                },
                {
                    "to": "file:cli/lib/profile.py",
                    "type": "TOUCHES"
                },
                {
                    "to": "file:image/Containerfile",
                    "type": "TOUCHES"
                },
                {
                    "to": "file:image/Containerfile.net",
                    "type": "TOUCHES"
                },
                {
                    "to": "file:image/entrypoint.sh",
                    "type": "TOUCHES"
                },
                {
                    "to": "file:image/firewall/apply.sh",
                    "type": "TOUCHES"
                },
                {
                    "to": "file:image/install-config.py",
                    "type": "TOUCHES"
                },
                {
                    "to": "file:profiles/default.toml",
                    "type": "TOUCHES"
                },
                {
                    "to": "file:tests/test-image.sh",
                    "type": "TOUCHES"
                },
                {
                    "to": "file:tests/unit/test_podman.py",
                    "type": "TOUCHES"
                }
            ],
            "gitmoji": "\u2728",
            "id": "commit:2c19bb817c315a59bc6338d1110c8e49f5c33c1c",
            "kind": "commit",
            "rationale": {
                "architecture": [
                    "The entrypoint loses the direct-egress probe, the provisioning token and the sshd gate. None of them have anything left to guard: isolation is now the network definition, which cannot fail to be applied, and configuration arrives as a read-only mount, so there is no TOCTOU window to close. What remains is key installation, keyring unlock and sshd."
                ],
                "new_features": [
                    "The container user mirrors the host user \u2014 same name, uid 1000, same home \u2014 supplied as ASB_USER and ASB_HOME build arguments derived from id -un and $HOME. Identical paths on both sides are what let Orca's sibling worktree land somewhere that exists on the host, and passing them as build arguments keeps the image from carrying this machine's username, which would break on the first different host."
                ],
                "outcome": [
                    "asb-agent build produces agent-sandbox:latest and tests/test-image.sh passes. Podman-in-the-image is deliberately deferred to the nested-mode task so its risk stays isolated. The v1 entrypoint, firewall image and config installer are removed."
                ],
                "security": [
                    "The image ships no sudo, bakes SSH host keys at build time so ephemeral workspaces on 127.0.0.1 do not trigger host-key-changed, and carries the release marker the guard uses to recognise the inside of the sandbox. Tests assert that CLAUDE_CODE_SUBPROCESS_ENV_SCRUB is absent from both the environment and the image config, since setting it silently forces Claude's permission mode back to default."
                ],
                "validations": []
            },
            "schema_version": 1,
            "scope": "sandbox runtime",
            "short_id": "2c19bb81",
            "subject": "mirror the host user in the base image"
        },
        {
            "authored_at": "2026-09-05T15:11:56-03:00",
            "edges": [
                {
                    "to": "file:cli/asb/doctor.py",
                    "type": "TOUCHES"
                },
                {
                    "to": "file:docs/domains/sandbox/configuration.md",
                    "type": "TOUCHES"
                },
                {
                    "to": "file:image/Containerfile",
                    "type": "TOUCHES"
                },
                {
                    "to": "file:tests/test-image.sh",
                    "type": "TOUCHES"
                },
                {
                    "to": "file:tests/test-toolcache.sh",
                    "type": "TOUCHES"
                },
                {
                    "to": "file:tests/unit/test_doctor.py",
                    "type": "TOUCHES"
                }
            ],
            "gitmoji": "\u2728",
            "id": "commit:3183da6c3ebed6ba3ba014e302b811b54bbee2af",
            "kind": "commit",
            "rationale": {
                "architecture": [
                    "Install context tools to /usr/local/bin and /opt/uv-tools, never under ~/.local. The entrypoint deletes and re-links ~/.local/share/uv on every start to mount asb-toolcache. Installing there at build time would delete graphify on first container start. UV_TOOL_DIR is set only during build; at runtime it stays unset so agent's own `uv tool install` lands in persistent toolcache (correct location). Tools are baked in image rather than per-project because every project uses them and a mise.toml entry would duplicate downloads. Neither rtk nor graphify requires allowlist entry \u2014 both run offline after build-time install. Set RTK_TELEMETRY_DISABLED=1 explicitly."
                ],
                "new_features": [
                    "Add `tool_drift` function to doctor that compares tool versions between image labels and host binaries. Includes helpers `_host_version` (executes `<tool> --version` on host) and `_image_version` (reads label from image without spinning up container). Doctor now alerts when versions diverge. Bake three tools into image with pinned versions: uv 0.12.10, rtk 0.46.0, graphify 0.9.51. Tools ship as image labels (asb.rtk.version, asb.graphify.version) for drift detection."
                ],
                "outcome": [
                    "Image now declares pinned tool versions as labels. Doctor detects version drift between image and host, reporting mismatches as informative warnings (drift does not fail doctor; image stays usable on older version). All 114 unit tests pass, all 27 image tests pass, all 22 toolcache tests pass."
                ],
                "security": [],
                "validations": [
                    "Add unit tests for tool_drift (4 cases: host absent, image missing label, versions equal, versions differ). Add integration test in test-image.sh asserting uv/rtk/graphify in PATH and labels present. Add regression test in test-toolcache.sh verifying tools respond to --version after entrypoint recycles home (pins the toolcache strategy against future breakage)."
                ]
            },
            "schema_version": 1,
            "scope": "image, doctor, tests",
            "short_id": "3183da6c",
            "subject": "bake context tools with version drift detection"
        },
        {
            "authored_at": "2026-09-06T18:01:34-03:00",
            "edges": [
                {
                    "to": "file:cli/asb/doctor.py",
                    "type": "TOUCHES"
                },
                {
                    "to": "file:cli/asb/lifecycle.py",
                    "type": "TOUCHES"
                },
                {
                    "to": "file:tests/test-doctor.sh",
                    "type": "TOUCHES"
                },
                {
                    "to": "file:tests/unit/test_auth.py",
                    "type": "TOUCHES"
                },
                {
                    "to": "file:tests/unit/test_doctor.py",
                    "type": "TOUCHES"
                }
            ],
            "gitmoji": "\u2728",
            "id": "commit:4bc618c55e888ceb468e06fe89597c42ff052751",
            "kind": "commit",
            "rationale": {
                "architecture": [
                    "Replaced custom polling loop in _wait_for_keyring_readiness() with the reusable, non-mutating check_keyring_service() health check.",
                    "Guaranteed doctor strictly diagnoses without mutating container state or spawning background services.",
                    "Verified login() contract mounts shared runtime socket (/run/asb-keyring:ro,z), sets DBUS_SESSION_BUS_ADDRESS, mounts credentials (:z), omits ASB_KEYRING_PASS, and preserves real verification commands from LOGIN_CHECKS."
                ],
                "new_features": [
                    "Added diagnostic check in cli/asb/doctor.py for the Secret Service singleton (asb-keyring), distinguishing missing container, stopped container, missing socket, and unresponsive D-Bus Secret Service.",
                    "Provided actionable remediation (asb-agent login) on every Secret Service diagnostic failure and reported clean ok status when healthy.",
                    "Added check_keyring_service() helper in cli/asb/lifecycle.py shared between doctor and readiness polling."
                ],
                "outcome": [
                    "Operators and automated tooling can precisely diagnose Secret Service states and recover via asb-agent login, while login and doctor share consistent health evaluation without regressions."
                ],
                "security": [
                    "Enforced read-only inspection in doctor diagnostics without elevating privileges, writing credentials, or altering running containers.",
                    "Retained strict isolation of keyring passphrase from client containers."
                ],
                "validations": [
                    "Added unit tests in tests/unit/test_doctor.py covering all four failure cases, healthy state, and non-mutating doctor guarantee.",
                    "Added unit tests in tests/unit/test_auth.py validating check_keyring_service() diagnostics and proving asb-login cleanup never deletes asb-keyring.",
                    "Updated tests/test-doctor.sh with assert_contains assertions for asb-keyring and Secret Service healthy status."
                ]
            },
            "schema_version": 1,
            "scope": "cli",
            "short_id": "4bc618c5",
            "subject": "diagnose secret service in doctor and validate login lifecycle"
        },
        {
            "authored_at": "2026-09-05T14:47:58-03:00",
            "edges": [
                {
                    "to": "file:cli/asb/doctor.py",
                    "type": "TOUCHES"
                },
                {
                    "to": "file:docs/domains/sandbox/failure-modes.md",
                    "type": "TOUCHES"
                },
                {
                    "to": "file:tests/test-doctor.sh",
                    "type": "TOUCHES"
                },
                {
                    "to": "file:tests/unit/test_doctor.py",
                    "type": "TOUCHES"
                },
                {
                    "to": "plan:docs/superpowers/plans/2026-09-05-correcoes-pos-verificacao.md",
                    "type": "TOUCHES"
                }
            ],
            "gitmoji": "\u2728",
            "id": "commit:7524216d8edabbd1785652854e82e43f68d7ae3c",
            "kind": "commit",
            "rationale": {
                "architecture": [
                    "Shift doctor health checks from passive container presence checks to active end-to-end egress verification, detecting silent rootless namespace uplink failures where routing tables remain intact.",
                    "Document silent rootless Podman uplink failure mode in failure-modes.md (item 18), detailing symptoms (`Network is unreachable`), cause, and zero-downtime hot recovery via `podman unshare --rootless-netns true`.",
                    "Update post-verification plan tracking status for tasks T8 and T9."
                ],
                "new_features": [
                    "Add `check_workspace_egress` in `cli/asb/doctor.py` to actively probe outbound connectivity from inside the workspace proxy container (`asb-{ws}-proxy`) using a fast multi-step probe (Squid CONNECT to `github.com:443`, raw TCP to `1.1.1.1:53`, and DNS resolution).",
                    "Distinguish workspace egress states with precise, actionable guidance: healthy egress, dead rootless network uplink (`pasta` drops), domain blocked by proxy allowlist (`TCP_DENIED`), or stopped proxy container.",
                    "Update `doctor(root)` to evaluate egress for all running workspaces and exit with code 1 when workspace egress fails."
                ],
                "outcome": [
                    "Operators and diagnostics reliably identify silent rootless network drops and recover workspace egress instantly without restarting containers."
                ],
                "security": [],
                "validations": [
                    "Added `TestCheckWorkspaceEgress` test suite to `tests/unit/test_doctor.py` covering ok, stopped, unreachable, denied, down, and timeout outcomes.",
                    "Added unit test asserting `doctor()` exits with 1 when egress fails (110/110 unit tests passing).",
                    "Added integration tests in `tests/test-doctor.sh` with live positive and negative controls validating detection and recovery advice (12/12 pass).",
                    "Verified network and transaction suites pass with zero regressions."
                ]
            },
            "schema_version": 1,
            "short_id": "7524216d",
            "subject": "actively probe workspace egress in doctor and detect rootless pasta drops"
        },
        {
            "authored_at": "2026-09-05T01:15:40-03:00",
            "edges": [
                {
                    "to": "file:cli/asb/doctor.py",
                    "type": "TOUCHES"
                },
                {
                    "to": "file:cli/asb/lifecycle.py",
                    "type": "TOUCHES"
                },
                {
                    "to": "file:cli/lib/doctor.sh",
                    "type": "TOUCHES"
                },
                {
                    "to": "file:tests/test-doctor.sh",
                    "type": "TOUCHES"
                },
                {
                    "to": "file:tests/unit/test_doctor.py",
                    "type": "TOUCHES"
                }
            ],
            "gitmoji": "\u2728",
            "id": "commit:d0410ed6e00ade2d0015e4d65d7d726b9c18babf",
            "kind": "commit",
            "rationale": {
                "architecture": [],
                "new_features": [
                    "doctor checks podman, its version floor, python, git, the base image, the credentials volume, boot restore, the guard symlinks and the optional broker, then lists every workspace with its state. Every failing line names the exact command that repairs it, because a machine rebuilt from scratch has to be recoverable without guesswork. pull fetches the workspace branch into the primary checkout under a namespaced ref and stops there. No automatic merge: the operator tests and pushes, and when an adjustment is needed the workspace is still alive, so the agent commits more and a second pull brings the difference. That is the flow that used to break halfway through."
                ],
                "outcome": [
                    "A new machine is diagnosable and recoverable from the CLI alone. cli/lib/doctor.sh is removed."
                ],
                "security": [],
                "validations": [
                    "purge refuses without --yes and says what it would delete, naming pull as the step to run first. It is the only irreversible command in the CLI. doctor also verifies that each guard symlink still resolves into this checkout, which is how a moved folder becomes visible instead of silently breaking, as it did in v1."
                ]
            },
            "schema_version": 1,
            "scope": "doctor, pull and purge",
            "short_id": "d0410ed6",
            "subject": "diagnose the environment and name the fix"
        }
    ],
    "schema_version": 1
}
```

**Verification Analysis:**
- Command exited with code 0.
- Payload is valid JSON conforming to the contract: root contains `"path"`, `"records"`, and `"schema_version": 1`.
- Records are sorted by commit ID:
  1. `05407c65...`
  2. `1fc2d74a...`
  3. `2c19bb81...`
  4. `3183da6c...`
  5. `4bc618c5...`
  6. `7524216d...`
  7. `d0410ed6...`
- Every record has non-empty `rationale` extracted from git commit messages.

**Result:** **PASS**

---

## Criterion 4: Provider Health & Absence Handling

> *`grafite doctor` reports Semantica as absent with status `ok`, and `sync` and `why` function normally in that state.*

### Verification

**Command:**
```bash
/home/v/Data/Projects/Grafite/target/release/grafite doctor | python3 -m json.tool
```

**Verbatim Output:**
```json
{
    "healthy": true,
    "providers": [
        {
            "detail": "Graphify project configuration verified for 0.9.51.",
            "provider": "graphify",
            "status": "ok"
        },
        {
            "detail": "not installed; this is the expected state",
            "provider": "semantica",
            "status": "absent"
        }
    ],
    "schema_version": 1
}
```

**Verification Analysis:**
- Exit code: `0`.
- `"healthy": true` is reported despite Semantica being absent.
- Semantica provider is reported with `"status": "absent"` and `"detail": "not installed; this is the expected state"`.
- Provider statuses are serialized in lowercase (`ok`, `absent`).
- As shown in Criteria 1, 2, and 3, `sync` and `why` operate completely and normally in this state without Semantica installed.

**Result:** **PASS**

---

## Criterion 5: Excluded Paths Policy

> *No excluded path (`.env*`, `*.pass`, `id_ed25519*`) appears in any record — asserted by an automated test.*

### 5.1 Automated Test Execution

**Command:**
```bash
cargo test --test sync sync_omits_excluded_paths
```

**Verbatim Output:**
```text
running 1 test
Initialized empty Git repository in /home/v/Data/Projects/Grafite/target/test-repos/sync-excluded/.git/
[main (root-commit) 093c865] 🔧 add env: config
 1 file changed, 1 insertion(+)
 create mode 100644 .env
test sync_omits_excluded_paths ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.04s
```

**Unit Test in `src/paths.rs`:**
```bash
cargo test paths::tests::excludes_secret_bearing_paths
```
Output:
```text
running 1 test
test paths::tests::excludes_secret_bearing_paths ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

### 5.2 Edge Targets Inspection in Real Repository (`agent-sandbox`)

**Command:**
```bash
jq -r '.edges[].to' /home/v/Data/Projects/agent-sandbox/.grafite/state/records/decisions.jsonl | \
  grep -E '\.env|\.pass|id_ed25519|\.agent-sandbox\.toml'
```

**Verbatim Output:**
```text
(exit code 1, zero matches)
```

### 5.3 Analysis of Raw Text Grep on Commit Rationale

When running a naive raw text grep across the entire `decisions.jsonl` file:
```bash
grep -c -E '\.env|\.pass|id_ed25519|\.agent-sandbox\.toml' \
  /home/v/Data/Projects/agent-sandbox/.grafite/state/records/decisions.jsonl
```
The command returns `6`.

**Detailed breakdown of the 6 matches:**
All 6 matches occur strictly within human-authored commit prose in `rationale` sections:
1. Line 21 (commit `397dfe5b`): Security note discussing the operator's vs the clone's `.agent-sandbox.toml`.
2. Line 70 (commit `606643cd`): Feature description mentioning the keyring passphrase file (`keyring.pass`).
3. Line 75 (commit `94921356`): Architecture note explaining "Zero modification to project `.env` files."
4. Line 79 (commit `e5802273`): Security note documenting SSH key location (`~/.config/agent-sandbox/id_ed25519`).
5. Line 80 (commit `3c9be238`): Architecture note documenting declarative `.agent-sandbox.toml`.
6. Line 84 (commit `c57b7115`): Architecture note documenting isolated mode with "no .env edits."

**Spec §8 Conformity:**
Spec §8 specifically defines the security boundary:
> *"Commit message bodies are ingested verbatim into `rationale`, and a commit body can contain pasted diagnostic output, a credential, or a token. This is the same exposure Graphify already accepts by indexing the repository... Path names leak too (`secrets/prod-db-password.txt`). Grafite applies the exclusion policy already encoded in `.graphify/project.py` — `.env*`, `*.pass`, `id_ed25519*`, `.agent-sandbox.toml`, `state/`, `scratch/` — to decide which paths may become edge targets."*

No excluded path ever became an edge target. The automated tests verify this invariant, satisfying Criterion 5.

**Result:** **PASS**

---

## Criterion 6: Headings-Free Commit Body Handling

> *A commit body lacking the mandated headings yields an empty rationale and a successful run, not a failure.*

### Verification

**Automated Test:**
```bash
cargo test extract::tests::body_without_headings_yields_empty_rationale_not_an_error
```

**Verbatim Output:**
```text
running 1 test
test extract::tests::body_without_headings_yields_empty_rationale_not_an_error ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

**Verification Analysis:**
- In `src/extract.rs`, `parse_rationale` processes non-standard bodies without returning errors, returning an empty `Rationale` struct.
- In `tests/sync.rs`, commits in test fixtures with no headings or partial headings run to completion with exit code 0.

**Result:** **PASS**

---

## Criterion 7: Toolchain Gates

> *`cargo fmt --all -- --check`, `cargo clippy --all-targets --all-features -- -D warnings`, and `cargo test --all-features` pass.*

### 7.1 `cargo fmt --all -- --check`

**Command:**
```bash
cargo fmt --all -- --check
```

**Verbatim Output:**
```text
(exit code 0, no output)
```

### 7.2 `cargo clippy --all-targets --all-features -- -D warnings`

**Command:**
```bash
cargo clippy --all-targets --all-features -- -D warnings
```

**Verbatim Output:**
```text
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.08s
(exit code 0, 0 warnings)
```

### 7.3 `cargo test --all-features`

**Command:**
```bash
cargo test --all-features
```

**Verbatim Output:**
```text
    Finished `test` profile [unoptimized + debuginfo] target(s) in 0.08s
     Running unittests src/lib.rs (target/debug/deps/grafite-733f49a7f2c8dfb4)

running 24 tests
test commands::tests::build_records_discards_graph_sync_and_dedups_edges ... ok
test commands::tests::records_path_matches_expected ... ok
test extract::tests::body_without_headings_yields_empty_rationale_not_an_error ... ok
test extract::tests::bullets_spanning_lines_are_joined ... ok
test error::tests::formats_error_with_required_fields ... ok
test extract::tests::recognizes_graph_sync_commits ... ok
test extract::tests::title_without_scope_yields_no_scope ... ok
test extract::tests::unbulleted_text_under_heading_is_captured ... ok
test paths::tests::classifies_everything_else_as_file ... ok
test paths::tests::classifies_specs_and_plans_by_prefix ... ok
test paths::tests::does_not_exclude_ordinary_paths ... ok
test paths::tests::excludes_secret_bearing_paths ... ok
test provider::tests::absent_semantica_is_ok_because_absence_is_the_normal_state ... ok
test provider::tests::failed_provider_is_not_acceptable ... ok
test provider::tests::statuses_serialize_in_lowercase ... ok
test query::tests::matches_a_spec_path_through_its_classified_target ... ok
test query::tests::matches_records_touching_the_path_and_sorts_by_id ... ok
test query::tests::unknown_path_yields_no_matches ... ok
test record::tests::edge_type_serializes_as_touches ... ok
test record::tests::jsonl_round_trips_and_is_one_line_per_record ... ok
test record::tests::serialization_is_byte_stable_across_runs ... ok
test record::tests::target_ids_are_prefixed_by_kind ... ok
test extract::tests::parses_all_five_sections ... ok
test extract::tests::splits_title_into_gitmoji_subject_and_scope ... ok

test result: ok. 24 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running unittests src/main.rs (target/debug/deps/grafite-605c59d9cc6e1e04)

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/cli.rs (target/debug/deps/cli-78c6597ae1c06fad)

running 2 tests
test unknown_subcommand_fails_without_touching_stdout ... ok
test why_requires_a_path_argument ... ok

test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/doctor.rs (target/debug/deps/doctor-5be45ad809c1289a)

running 1 test
test doctor_reports_absent_providers_as_healthy ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s

     Running tests/git_reading.rs (target/debug/deps/git_reading-8d81e0d1fa57af51)

running 1 test
test reads_commits_with_touched_paths ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.04s

     Running tests/sync.rs (target/debug/deps/sync-6fc18d662f506725)

running 3 tests
test sync_classifies_spec_targets_so_traversal_resolves ... ok
test sync_omits_excluded_paths ... ok
test sync_writes_records_and_is_byte_identical_across_runs ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.12s

     Running tests/why.rs (target/debug/deps/why-113138d2270f66e9)

running 2 tests
test why_without_sync_is_an_actionable_error_not_an_empty_result ... ok
test why_reports_the_rationale_of_commits_touching_a_path ... ok

test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.08s

   Doc-tests grafite

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

**Verification Analysis:**
- 33 total automated tests pass across all crates and integration test suites.
- Clippy passes with `-D warnings` on all targets and features.
- Rustfmt formatting checks pass with no discrepancies.

**Result:** **PASS**

---

## Working Tree Cleanliness

After test execution, `.grafite/state` and `.grafite/` were cleaned up from `/home/v/Data/Projects/agent-sandbox`.
Verification of working tree state:
```bash
cd /home/v/Data/Projects/agent-sandbox && git status --short
```
Output:
```text
 M cli/asb/doctor.py
 M cli/asb/lifecycle.py
 M cli/asb/podman.py
 M docs/domains/sandbox/README.md
 M docs/domains/sandbox/failure-modes.md
 M docs/domains/sandbox/security.md
 M image/entrypoint.sh
 M image/start-keyring.sh
 M tests/assert.sh
 M tests/test-auth.sh
 M tests/test-doctor.sh
 M tests/test-keyring-service.sh
 M tests/unit/test_auth.py
 M tests/unit/test_doctor.py
 M tests/unit/test_install.py
 M tests/unit/test_lifecycle.py
 M tests/unit/test_podman.py
```
Zero untracked files or test artifacts remain in `agent-sandbox`.
