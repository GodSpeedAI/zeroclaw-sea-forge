# Copilot Instructions for SEA-Forge ZeroClaw

## Purpose

You are working in the SEA-Forge fork of ZeroClaw.

Upstream ZeroClaw is the Rust agent runtime. SEA-Forge adds governance: authority onboarding, action evaluation, correlation context, policy files, sidecar installation, and restore metadata.

Your job is to preserve upstream behavior while protecting SEA's fork-specific enforcement points.

## Instruction priority

1. Direct user request
2. Safety, secrets, and environment constraints
3. This file
4. Upstream ZeroClaw docs, tests, and code
5. Local code patterns
6. General Rust and agent-runtime best practices

When code and docs conflict, trust the code after inspection.

## Fork invariants

- Credit upstream ZeroClaw. Do not imply this fork is the official ZeroClaw project.
- Keep SEA authority checks before side effects.
- Do not remove SEA onboarding or evaluation calls.
- Do not remove SEA correlation, workspace, runtime, policy-bundle, or audit context.
- Do not bypass SEA policy with fallback local execution after `deny` or `escalate`.
- Do not replace reversible sidecar install behavior with destructive in-place patching.
- Do not remove restore manifests, backup creation, pinned release metadata, or policy examples.
- Do not hide degraded governance. If the authority is unreachable, follow the configured fail-closed or degraded-mode policy.

## Protected SEA surfaces

Treat these concepts as fork-owned even when upstream changes nearby code:

- `/policy/authority/onboard`
- `/policy/authority/evaluate`
- `SEA_AUTHORITY_URL`
- `SEA_CORRELATION_ID`
- `SEA_SOURCE_PLATFORM=zeroclaw`
- `SEA_WORKSPACE_CONFIG`
- Canonical Action Model mapping for file, shell, HTTP, web, and merge actions
- before-side-effect checks for `file_write`, `file_edit`, `shell`, `http_request`, and `web_fetch`
- sidecar install path: `~/.sea/zeroclaw/bin/zeroclaw`
- install manifest: `~/.sea/zeroclaw/install-manifest.json`
- workspace wrapper: `.sea/bin/sea-zeroclaw`
- policy directory: `~/.sea/zeroclaw/policies/`

## Merge conflict protocol

When syncing with upstream:

1. Run `git status --short` and identify every conflicted file.
2. Classify each conflict as upstream-owned, SEA-owned, or mixed.
3. For mixed files, inspect both sides before editing. Preserve upstream fixes and reapply SEA authority hooks around the new upstream structure.
4. Never use `git checkout --theirs`, `git checkout --ours`, or editor "accept all" actions on files that contain SEA enforcement, install, policy, or manifest logic.
5. After resolving, search the touched files for SEA terms listed above. If a protected term disappeared, verify that the behavior moved elsewhere before accepting the resolution.
6. Re-run focused tests for the touched area.
7. In the final note, state which upstream changes were kept and which SEA fork changes were preserved.

If an upstream refactor removes the call site where SEA checks used to run, do not invent a silent bypass. Find the new side-effect boundary and put the authority check there.

## Coding rules

- Inspect before editing.
- Keep changes small and tied to the requested task.
- Match upstream style for Rust, TOML, shell, and docs.
- Prefer existing traits, adapters, and config surfaces over new parallel systems.
- Do not add compatibility shims unless the user explicitly asks and the tradeoff is documented.
- Do not log secrets, API keys, provider tokens, OAuth device codes, or raw request bodies.
- Keep user config local. Do not exfiltrate ZeroClaw config, memory, receipts, or workspace files.

## Documentation rules

- Keep upstream ZeroClaw credit visible.
- Describe SEA behavior as a fork or integration, not official upstream behavior.
- Do not include GitHub star-history graphs or contributor image blocks in the fork README.
- Update docs when behavior, commands, config paths, or policy semantics change.

## Validation

Use the smallest meaningful checks for the change.

For Rust runtime changes:

```bash
cargo fmt --check
cargo clippy --workspace --all-targets
cargo test --workspace
```

For SEA integration changes from the SEA-Forge repository:

```bash
python3 -m pytest tests/integration/test_zeroclaw_release_manifest.py tests/integration/test_zeroclaw_adopt_existing_install.py -q
python3 -m tools.sea_integrate doctor --runtime zeroclaw --workspace . --authority-url "$SEA_AUTHORITY_URL" --json
```

For installer changes:

```bash
bash integrations/zeroclaw/install.sh --dry-run
bash integrations/zeroclaw/uninstall.sh --workspace . --dry-run
```

If a command cannot run locally, say why and name the next command the maintainer should run.

## Done criteria

- Upstream behavior still works or the intentional difference is documented.
- SEA checks still occur before side effects.
- Fork-owned install, policy, manifest, and restore paths are preserved.
- Relevant tests or dry-runs were executed, or the reason they were not run is explicit.
- The result is classified honestly: `authority-only`, `local-confidence`, `focused-slice`, `live-dev-proof`, or `release-gate-proof`.

