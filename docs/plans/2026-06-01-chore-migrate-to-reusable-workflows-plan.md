---
title: "chore: migrate CI/release to brettdavies/.github reusable workflows"
type: chore
date: 2026-06-01
status: blocked
---

# Migrate CI/Release to Reusable Workflows

## Goal

Replace the inline `ci.yml` + `release.yml` workflows with thin callers of `brettdavies/.github`'s `rust-ci.yml` +
`rust-release.yml` reusable workflows, per the github-repo-setup standard.

## Why This Is Blocked

Two structural mismatches between the current PAYG workflows and the standard reusables prevent a drop-in swap:

### 1. Feature-matrix coverage gap

PAYG's `ci.yml` runs the test job across three feature combinations:

- `""` (default — `x402` only)
- `--no-default-features --features eth`
- `--all-features`

The standard `rust-ci.yml` reusable runs a single feature configuration (typically `--all-features` for the test job, or
whatever the consuming repo sets via inputs). Migrating as-is would silently lose coverage of the `x402`-only and
`eth`-only feature combinations — exactly the matrix PAYG exists to validate.

**Resolution path:** Either extend `rust-ci.yml` upstream to accept a `features-matrix` input, OR keep a small inline
`features` job in PAYG's caller for the matrix while delegating fmt/clippy/audit to the reusable.

### 2. Release model mismatch

PAYG uses `release-plz` (automated Release PR; merge PR → tag + GitHub release). The standard `rust-release.yml`
reusable implements the draft-then-finalize pattern (push tag → draft release → Homebrew bottles build →
`finalize-release` publishes). These are incompatible release pipelines, not variants of the same one.

Switching to `rust-release.yml` would mean:

- Removing `release-plz.toml` and the release-plz automation
- Adding manual version bump + tag-push workflow
- Configuring the `homebrew-tap` dispatch chain
- Adopting the `release/*` cherry-pick branch pattern

That is a release-process change, not a workflow refactor.

**Resolution path:** Two valid futures —

- **Keep release-plz**, document PAYG as an exception in the standard, and do not migrate `release.yml`.
- **Adopt the standard release model** — separate decision; requires a full plan including changelog migration, homebrew
  formula setup, and the `release/*` branch pattern with `guard-main-docs.yml` + `guard-release-branch.yml` guard
  workflows.

## Adjacent Items Still Worth Doing Without Full Migration

These are independent of the migration question and should land regardless:

1. Add `FORCE_JAVASCRIPT_ACTIONS_TO_NODE24: true` env to both workflows (June 2026 deadline).
2. Let open Dependabot PRs land (`Swatinem/rust-cache@v2.9.1`, `cargo-deny-action@v2.0.15`).
3. Bump `actions/checkout` in `release.yml` from v4.3.1 to v6.0.2 to match `ci.yml`.

## Required Status Checks Impact

The Protect-main ruleset currently requires PAYG-specific check contexts:

- `test ()` / `test (--no-default-features --features eth)` / `test (--all-features)`
- `lint`, `msrv`, `deny`, `doc`

Any caller-workflow migration must update `.github/rulesets/protect-main.json` in lockstep — otherwise the ruleset waits
forever on contexts that never report. See
`docs/solutions/integration-issues/github-status-check-context-inline-vs-reusable-2026-04-14.md` in solutions-docs for
the published-context pitfall.
