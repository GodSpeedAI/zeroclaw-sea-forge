# SEA-Forge ZeroClaw

### Run ZeroClaw with SEA authority enforcement before high-impact actions execute.

This is the SEA-Forge fork and integration layer for [ZeroClaw](https://github.com/zeroclaw-labs/zeroclaw). Upstream ZeroClaw is a Rust single-binary personal assistant runtime with pluggable model providers, channels, tools, services, security policy, SOPs, ACP support, and local-first operation. SEA-Forge keeps that native runtime model and adds policy evaluation, workspace provenance, and reversible sidecar installation.

This repository is not the official ZeroClaw project. Credit for ZeroClaw belongs to ZeroClaw Labs and its maintainers. The SEA fork exists to connect ZeroClaw to SEA-Forge governance.

---

## Three guarantees

- **ZeroClaw keeps its native UX.** Users still run ZeroClaw as ZeroClaw. SEA adds a workspace wrapper and sidecar path instead of replacing the agent workflow.
- **SEA evaluates before side effects.** File writes, edits, shell execution, HTTP requests, and web fetches are mapped into SEA authority checks before the action runs.
- **Fork changes are reversible and auditable.** The installer snapshots existing ZeroClaw state, writes a restore manifest, and keeps SEA-managed runtime state under `~/.sea/zeroclaw/`.

---

## 60-second quickstart

```bash
# ZeroClaw must already be installed and available on PATH.
zeroclaw --version

# From the SEA-Forge repository:
export SEA_AUTHORITY_URL=http://localhost:8080
bash integrations/zeroclaw/install.sh

# Verify the workspace wrapper and authority connection:
.sea/bin/sea-zeroclaw --help
python3 -m tools.sea_integrate doctor --runtime zeroclaw --workspace . --authority-url "$SEA_AUTHORITY_URL" --json
```

Default policies are copied to:

```text
~/.sea/zeroclaw/policies/
```

Edit those policies to control file paths, PR merge rules, and API egress for your workspace.

---

## What changes from upstream ZeroClaw

Upstream ZeroClaw provides the agent runtime: providers, channels, tools, local config, services, and security defaults.

SEA-Forge adds:

- workspace metadata in `.sea/workspace.yaml`
- a runtime wrapper at `.sea/bin/sea-zeroclaw`
- SEA correlation and workspace environment variables
- default policy files under `~/.sea/zeroclaw/policies/`
- a SEA-managed sidecar binary under `~/.sea/zeroclaw/bin/zeroclaw`
- install and restore metadata under `~/.sea/zeroclaw/install-manifest.json`
- authority calls to `/policy/authority/onboard` and `/policy/authority/evaluate` when `SEA_AUTHORITY_URL` is set

Governed tools in this fork:

- `file_write`
- `file_edit`
- `shell`
- `http_request`
- `web_fetch`

The authority decision is one of:

```text
allow | deny | escalate
```

---

## Install model

The installer is adoption-first. If ZeroClaw already exists, SEA records the current binary, config, and service files before activating the managed sidecar.

Typical observed upstream paths:

```text
~/.zeroclaw/config.toml
~/.config/systemd/user/zeroclaw.service
$(command -v zeroclaw)
```

SEA-managed paths after install:

```text
.sea/workspace.yaml
.sea/integration.json
.sea/.install-manifest.json
.sea/bin/sea-zeroclaw
~/.sea/zeroclaw/bin/zeroclaw
~/.sea/zeroclaw/install-manifest.json
~/.sea/zeroclaw/backups/<timestamp>/
~/.sea/zeroclaw/policies/file-access-policy.yaml
~/.sea/zeroclaw/policies/pr-merge-policy.yaml
~/.sea/zeroclaw/policies/api-allowlist.yaml
```

The pinned SEA fork source is recorded in [`release.json`](release.json). Keep that file current when rebasing or syncing the fork.

---

## How enforcement works

```text
ZeroClaw tool request
        |
SEA wrapper adds runtime, workspace, and correlation context
        |
Fork maps the native tool request into SEA's Canonical Action Model
        |
SEA authority evaluates declared policy
        |
allow: execute | deny: block | escalate: stop for approval
        |
decision and context are recorded for audit
```

The important invariant is order: SEA evaluation must happen before the tool performs the side effect. Do not move authority checks after writes, shell execution, HTTP calls, or merge actions.

---

## Verification

Run a focused integration check:

```bash
python3 -m tools.sea_integrate doctor --runtime zeroclaw --workspace . --authority-url "$SEA_AUTHORITY_URL" --json
```

Run the current automated checks for this integration:

```bash
uv run --directory services/policy-gateway --extra dev python -m pytest tests/test_authority_routes.py -q
cargo test --lib sea_authority -- --nocapture
cargo test --lib sea_forge -- --nocapture
python3 -m pytest tests/integration/test_zeroclaw_release_manifest.py tests/integration/test_zeroclaw_adopt_existing_install.py -q
```

What these checks prove:

- SEA authority onboarding and evaluation routes respond.
- The ZeroClaw authority client can onboard and evaluate actions.
- The governed tools still block before side effects.
- The installer records the pinned fork release metadata and can adopt an existing install.

This is `focused-slice` proof unless it also includes a live authority service, a real ZeroClaw binary, and evidence from an actual governed action.

---

## Uninstall

Restore the previous ZeroClaw state and remove the SEA workspace integration:

```bash
bash integrations/zeroclaw/uninstall.sh --workspace . --yes
```

Preview the restore without changing files:

```bash
bash integrations/zeroclaw/uninstall.sh --workspace . --dry-run
```

The uninstall flow reads `~/.sea/zeroclaw/install-manifest.json`, restores backed-up ZeroClaw files, runs `tools.sea_integrate uninstall`, and optionally purges backups with `--purge`.

---

## Maintaining the fork

Use [`./.github/copilot-instructions.md`](.github/copilot-instructions.md) as the agent operating contract for this fork. It tells coding agents how to preserve SEA changes during upstream syncs and merge conflicts.

The short version:

- Do not accept upstream conflict hunks wholesale in files that contain SEA authority hooks.
- Keep upstream ZeroClaw behavior unless it conflicts with SEA's before-side-effect enforcement.
- Preserve SEA wrapper, sidecar, release manifest, policy, and restore semantics.
- Credit upstream ZeroClaw clearly and avoid implying this fork is the official project.

---

## Upstream credit

ZeroClaw is built and maintained by the ZeroClaw community. See the official repository for upstream documentation, maintainers, contribution process, security policy, license terms, and current release information:

- <https://github.com/zeroclaw-labs/zeroclaw>
- <https://www.zeroclaw.dev/>
