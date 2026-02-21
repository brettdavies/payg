---
title: "Release Automation with release-plz in a Rust Workspace"
category: integration-issues
tags:
  - release-automation
  - github-actions
  - rust-workspace
  - ci-cd
  - supply-chain-security
  - cargo
  - conventional-commits
module: build-system
symptom: "No release infrastructure — manual version bumps, no CHANGELOG, no git tags, no GitHub Releases. Two crates must version in lockstep but have no mechanism to enforce it."
root_cause: "Zero automation. Manual releases across a multi-crate workspace are error-prone: deciding semver bumps, writing changelogs, bumping both Cargo.toml files, creating tags — all by hand."
date: 2026-02-17
pr_ref: "https://github.com/brettdavies/payg/pull/6"
---

# Release Automation with release-plz in a Rust Workspace

## Problem

A Rust workspace with 2 crates (`payg` lib + `payg-cli` binary) had no release infrastructure. Every release required manually deciding the semver bump, writing changelog entries, bumping `version` in both `Cargo.toml` files, creating git tags, and publishing GitHub Releases. Both crates must always share the same version, making manual coordination especially error-prone.

## Root Cause

No tooling existed. Conventional Commits were partially adopted (27% of commits) but not leveraged for automation. The workspace had no `[workspace.package]` for version inheritance, so versions were duplicated across crate manifests.

## Solution

Use **release-plz** — purpose-built for Rust workspaces. It leverages Conventional Commits for automatic semver bumps (`feat` → minor, `fix` → patch, `!` → major), generates changelogs via git-cliff, opens release PRs, and creates git tags + GitHub Releases.

### Key Configuration

**`release-plz.toml`** (project root):

```toml
[workspace]
git_only = true
release_always = false
semver_check = false

[[package]]
name = "payg"
version_group = "payg"
git_tag_name = "v{{ version }}"
changelog_path = "CHANGELOG.md"
changelog_include = ["payg-cli"]

[[package]]
name = "payg-cli"
version_group = "payg"
git_tag_enable = false
git_release_enable = false
changelog_update = false

[changelog]
sort_commits = "oldest"
protect_breaking_commits = true
commit_parsers = [
  { message = "^feat", group = "Features" },
  { message = "^fix", group = "Bug Fixes" },
  { message = "^docs", group = "Documentation" },
  { message = "^perf", group = "Performance" },
  { message = "^refactor", group = "Refactoring" },
  { message = "^test", group = "Testing" },
  { message = "^ci", skip = true },
  { message = "^chore", skip = true },
  { message = "^style", skip = true },
  { message = "^revert", group = "Reverted" },
]
```

**Workspace version inheritance** (`Cargo.toml` root):

```toml
[workspace.package]
version = "0.1.0"
edition = "2024"
rust-version = "1.88.0"
license = "MIT"
```

Both crates use `version.workspace = true` to inherit.

**Release workflow** (`.github/workflows/release.yml`): Two separate jobs (`release` and `release-pr`), all actions SHA-pinned, static concurrency groups, fork guard via `if: github.repository == 'brettdavies/payg'`, no cache step.

## Key Gotchas

### 1. Action path migrated — use `release-plz/action`, not `MarcoIeni/release-plz-action`

The release-plz GitHub Action was migrated from `MarcoIeni/release-plz-action` to `release-plz/action`. The old path is deprecated and may point to stale versions. Always check the [canonical repository](https://github.com/release-plz/action).

### 2. Use `git_only = true` for unpublished crates, not `publish = false`

`git_only = true` tells release-plz to determine versions from git tags instead of querying the crates.io registry. This is the correct setting for crates that are never published. `publish = false` is a different Cargo.toml field that tells `cargo publish` to refuse — it doesn't affect release-plz's version detection behavior.

### 3. Don't cache in release workflows

`Swatinem/rust-cache@v2` is unnecessary in release workflows. release-plz downloads a pre-built binary via cargo-binstall and never compiles your crates. The cache step wastes 20-60s per run and writes empty entries to the 10 GB per-repo cache budget.

### 4. Enable `can_approve_pull_request_reviews` in repo settings

This GitHub setting defaults to `false`. Without it, the `release-pr` job fails with "Resource not accessible by integration". Enable via: Settings > Actions > General > "Allow GitHub Actions to create and approve pull requests".

Verify current state:
```sh
gh api repos/{owner}/{repo}/actions/permissions/workflow
# Look for: "can_approve_pull_request_reviews": true
```

### 5. SHA-pin all third-party actions

Mutable tags (`@v4`, `@v0.5`) can be reassigned. CVE-2025-30066 (tj-actions incident) proved this is an active attack vector. Pin to immutable commit SHAs:

```yaml
# Correct — immutable
- uses: actions/checkout@11bd71901bbe5b1630ceea73d27597364c9af683 # v4.2.2

# Wrong — mutable tag
- uses: actions/checkout@v4
```

**Exception:** `dtolnay/rust-toolchain@stable` is intentionally unpinned because `@stable` is the intended semantic ("latest stable Rust"). Pinning to a SHA would freeze the Rust version.

Use Dependabot (`.github/dependabot.yml`) to keep SHA-pinned actions updated automatically.

### 6. Create an anchor tag before enabling release-plz

Without a tag boundary, release-plz scans all commits for the changelog. If Conventional Commits adoption is incomplete (ours was 27%), the first changelog will be noisy — freeform commits appear under a generic "Other" heading.

Create the tag on current `main` HEAD first:
```sh
git tag v0.1.0 main
git push origin v0.1.0
```

After this, release-plz only analyzes commits **after** the tag.

### 7. Use static concurrency group keys

When a workflow only triggers on one branch (e.g., `push: branches: [main]`), use static strings for concurrency groups:

```yaml
concurrency:
  group: release-plz-release    # static string
  cancel-in-progress: false
```

Not `release-plz-${{ github.ref }}` — the ref is always `refs/heads/main`. Static keys are simpler and eliminate expression injection surface.

### 8. `version_group` does NOT unconditionally bump all members

`version_group = "payg"` only cascades bumps when the dependency chain requires it. A change to the lib cascades to the CLI (because `payg-cli` depends on `payg`). But a change **only** to `payg-cli` bumps only `payg-cli`. This is correct behavior.

### 9. GITHUB_TOKEN cannot trigger workflows on its own PRs

PRs created by the default `GITHUB_TOKEN` won't trigger `on: pull_request` workflows. This means Release PRs don't get CI checks automatically. This is acceptable because:

1. Code in the Release PR was already tested when merged to `main`
2. The Release PR only changes `Cargo.toml` versions and `CHANGELOG.md`
3. If CI-on-release-PRs is needed later, switch to a GitHub App token

### 10. Merge Release PRs with merge commit, not squash

Squash merging a Release PR creates a race condition in release-plz's detection. The tool expects to find its own version bump commit in the history; squashing destroys that. Always use standard merge commit for Release PRs.

## Prevention Checklist

When setting up release-plz on a new Rust workspace:

- [ ] Verify action path is `release-plz/action` (not `MarcoIeni/...`)
- [ ] Set `git_only = true` for unpublished crates
- [ ] Omit `Swatinem/rust-cache` from release workflow
- [ ] Enable `can_approve_pull_request_reviews` in repo settings
- [ ] SHA-pin all third-party actions (except `dtolnay/rust-toolchain@stable`)
- [ ] Configure Dependabot for `github-actions` updates
- [ ] Create anchor tag at current version before enabling automation
- [ ] Use static concurrency group keys
- [ ] Document merge strategy (no squash on Release PRs) in CLAUDE.md or CONTRIBUTING.md
- [ ] Verify Conventional Commits adoption rate; enforce via CI if needed

## Related Documentation

- [CI feature matrix gotcha](../build-errors/rust-ci-feature-matrix-additive-gotcha.md) — CI configuration mistakes with Rust feature flags
- [Workspace dependency conflicts](../build-errors/rust-workspace-dependency-conflicts-and-feature-flags.md) — dependency pinning patterns to preserve during version bumps
- [Phase 3 review findings](../code-review/phase3-multi-agent-review-findings.md) — CI hardening baseline (permissions, fail-fast, actions pinning)

## External References

- [release-plz configuration reference](https://release-plz.dev/docs/config)
- [release-plz GitHub Action quickstart](https://release-plz.dev/docs/github/quickstart)
- [release-plz/action repository](https://github.com/release-plz/action)
- [release-plz token documentation](https://release-plz.dev/docs/github/token)
- [StepSecurity: Pinning GitHub Actions](https://www.stepsecurity.io/blog/pinning-github-actions-for-enhanced-security-a-complete-guide)
