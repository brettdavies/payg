---
title: "feat: versioning and release automation with release-plz"
type: feat
date: 2026-02-17
deepened: 2026-02-17
---

# Versioning & Release Automation with release-plz

## Enhancement Summary

**Deepened on:** 2026-02-17  
**Agents used:** security-sentinel, architecture-strategist, code-simplicity-reviewer, deployment-verification-agent, performance-oracle, pattern-recognition-specialist, best-practices-researcher, framework-docs-researcher, git-history-analyzer

### Key Improvements

1. **Supply chain security:** All actions pinned to immutable commit SHAs with Dependabot for updates (CVE-2025-30066 lesson)
2. **Action reference corrected:** `MarcoIeni/release-plz-action` is deprecated; canonical path is `release-plz/action`
3. **`git_only = true` added:** Correct mode for crates not publishing to crates.io — determines versions from git tags instead of querying the registry
4. **`Swatinem/rust-cache@v2` removed from release workflow:** release-plz downloads a pre-built binary and never compiles your crates — the cache step wastes ~20-60s per run and pollutes the 10 GB cache budget
5. **Concurrency hardened:** Both jobs now have concurrency groups with `cancel-in-progress: false`; static keys replace `${{ github.ref }}` since the workflow only triggers on `main`
6. **Changelog customization added:** `[changelog]` section with `commit_parsers` to group entries by type and skip noise (`ci:`, `chore:`, `style:`)
7. **Bootstrap procedure made prescriptive:** Create `v0.1.0` tag before enabling release-plz (git history analysis shows only 27% of commits use Conventional Commits — without a tag boundary the first changelog will be noisy)
8. **Blocking prerequisite surfaced:** `can_approve_pull_request_reviews` is currently `false` in repo settings — must be enabled or the release-pr job will fail

### Corrections from Original Plan

| Issue | Original | Corrected |
|---|---|---|
| Action path | `MarcoIeni/release-plz-action@v0.5` | `release-plz/action@<SHA>` (SHA-pinned) |
| Action pinning | Mutable tag references | Full SHA with version comments + Dependabot |
| Rust cache in release workflow | Included | Removed (unnecessary) |
| `git_only` | Not set | `true` (required for unpublished crates) |
| Release job concurrency | None | Added `cancel-in-progress: false` |
| Concurrency group key | `release-plz-${{ github.ref }}` | Static string (always `main`) |
| Changelog customization | None (defaults) | `[changelog]` section with commit_parsers |
| Bootstrap tag | Optional | Prescriptive (do it) |

---

## Overview

PAYG is at v0.1.0 with no release infrastructure — no CHANGELOG, no git tags, no GitHub Releases, no automation. The project has adopted Conventional Commits (recent commits follow the convention; earlier history does not) and has a `development` → `main` branch workflow. Both crates (lib + CLI) are tightly coupled and must always share the same version.

This plan adds [release-plz](https://release-plz.dev/) to automate version bumps, changelog generation, and GitHub Releases via a single GitHub Actions workflow.

## Problem Statement

Manual releases are error-prone, especially with two crates that must stay in lockstep. Without automation, every release requires:

1. Manually deciding the correct semver bump from commit history
2. Manually writing changelog entries
3. Manually bumping `version` in both `Cargo.toml` files
4. Manually creating git tags and GitHub Releases
5. Keeping both crates at the same version

This is work a machine should do, and Conventional Commits already contain all the information needed.

## Proposed Solution

Use **release-plz** — purpose-built for Rust workspaces. It leverages existing Conventional Commits for automatic version bumps (`feat` → minor, `fix` → patch, `!` → major), generates changelogs via git-cliff, opens release PRs, and creates git tags + GitHub Releases.

### Why not changesets?

Changesets is JS-only, requires a `package.json` shim, and solves independent-versioning in large JS monorepos. PAYG has neither constraint — 2 Rust crates in lockstep. Wrong tool.

### Why not cargo-release?

cargo-release is manual — you run `cargo release` locally. It doesn't integrate with CI, doesn't auto-generate changelogs from commits, and doesn't open PRs for review. release-plz does all of this.

## Versioning Scheme

| Decision | Choice | Rationale |
|---|---|---|
| Version format | Semver (major.minor.patch) | Rust ecosystem standard |
| Crate versioning | Lockstep via `version_group` | CLI wraps lib; independent versions add confusion, no value |
| Tag format | `v0.2.0` (single tag) | Not per-crate; lockstep makes per-crate tags redundant |
| Changelog | Single root `CHANGELOG.md` | Both crates' changes combined; two changelogs confuse users |
| crates.io | Not publishing yet (`git_only = true`) | Stable API not reached; defer per `docs/plans/2026-02-09-feat-payg-v1.1-future-enhancements-plan.md` |
| Semver checking | Disabled (`semver_check = false`) | Requires a published crate baseline on crates.io |

### Research Insights: `version_group` Behavior

`version_group` does **not** unconditionally bump all members. From the [release-plz docs](https://release-plz.dev/docs/config):

> "The version group is considered only when packages contain changes."

For PAYG this works correctly because `payg-cli` depends on `payg`. Any change to the lib cascades to the CLI via the dependency chain, so both always bump together. However, a change **only** to `payg-cli` (e.g., CLI argument tweak) will bump only `payg-cli`, not `payg`. This is the correct behavior — a CLI-only change shouldn't force a lib version bump.

### Research Insights: Pre-1.0 Semver

No breaking change markers (`!` or `BREAKING CHANGE:` footer) exist in the git history. A `feat!:` on 0.x would bump to 0.2.0 (minor), not 1.0.0. release-plz follows standard semver where pre-1.0 breaking changes are minor bumps. Risk of accidental 1.0.0 is zero with current commit history.

## Release Lifecycle

```
feature-branch → PR → development → PR → main
                                         ↓
                             release-plz opens Release PR
                             (bumped Cargo.toml + CHANGELOG.md)
                                         ↓
                             maintainer reviews & merges (merge commit, not squash)
                                         ↓
                             release-plz creates git tag + GitHub release
```

## Technical Approach

### Changes Summary (8 files touched, 3 new)

| # | File | Action | Purpose |
|---|---|---|---|
| 1 | `Cargo.toml` (root) | Edit | Add `[workspace.package]` for version inheritance |
| 2 | `crates/payg/Cargo.toml` | Edit | Inherit workspace metadata |
| 3 | `crates/payg-cli/Cargo.toml` | Edit | Inherit workspace metadata |
| 4 | `release-plz.toml` | **New** | release-plz configuration |
| 5 | `.github/workflows/release.yml` | **New** | GitHub Actions release workflow |
| 6 | `.github/dependabot.yml` | **New** | Automated action SHA updates |
| 7 | `.github/workflows/ci.yml` | Edit | Add `main` to PR trigger branches |
| 8 | `CLAUDE.md` | Edit | Document versioning conventions |

---

### 1. Add workspace version inheritance — `Cargo.toml` (root)

Insert `[workspace.package]` between `[workspace]` and `[profile.release]`:

```toml
[workspace.package]
version = "0.1.0"
edition = "2024"
rust-version = "1.88.0"
license = "MIT"
```

**Why:** Single source of truth for shared metadata (STAR principle). Both crates inherit instead of duplicating. release-plz bumps the workspace version once and both crates follow.

> **Simplicity note:** Only `version` inheritance is strictly required for release-plz. Including `edition`, `rust-version`, and `license` is a DRY improvement bundled with this change since the `[workspace.package]` section is being created anyway. If you prefer minimal scope, inherit only `version` and leave the others hardcoded.

### 2. Inherit in crate manifests

**`crates/payg/Cargo.toml`** — replace hardcoded `version`, `edition`, `rust-version`, `license`:

```toml
[package]
name = "payg"
version.workspace = true
edition.workspace = true
rust-version.workspace = true
description = "Pay-as-you-go crypto micropayments for CLI tools"
license.workspace = true
```

**`crates/payg-cli/Cargo.toml`** — same pattern:

```toml
[package]
name = "payg-cli"
version.workspace = true
edition.workspace = true
rust-version.workspace = true
description = "CLI for PAYG crypto micropayments"
license.workspace = true
```

The path dependency on `payg` stays as-is (`payg = { path = "../payg" }`). A `version` field is only required for crates.io publishing, which is out of scope. release-plz handles path dependencies correctly without it when `git_only = true`.

#### Research Insight: Defense in Depth

Consider adding `publish = false` to both crate `Cargo.toml` files as a belt-and-suspenders guard against accidental `cargo publish` invocations. This is separate from release-plz's `git_only = true` — the Cargo.toml `publish` field tells `cargo publish` itself to refuse.

### 3. Create `release-plz.toml` (new file, project root)

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

**Key decisions:**

| Setting | Value | Rationale |
|---|---|---|
| `git_only` | `true` | Determines versions from git tags, not crates.io registry. Required for unpublished crates. Replaces the old `publish = false` approach. |
| `release_always` | `false` | Enforces two-phase release: (1) release-pr opens PR, (2) only after PR merge does the release job create tags. Prevents accidental releases from direct pushes to main. Without this, any version bump on main would immediately trigger tag creation. |
| `semver_check` | `false` | No published baseline to check against; enable when first published |
| `version_group = "payg"` | Both crates | Ensures lockstep version bumps. Since `payg-cli` depends on `payg`, changes to the lib cascade to CLI. |
| `git_tag_name = "v{{ version }}"` | On `payg` only | Single tag per release, not per-crate |
| `git_tag_enable = false` | On `payg-cli` | Avoids duplicate tags; `payg` owns the single tag |
| `git_release_enable = false` | On `payg-cli` | Single GitHub Release from `payg` crate |
| `changelog_path = "CHANGELOG.md"` | On `payg` | Root-level changelog. Path is relative to root `Cargo.toml` (confirmed in [config docs](https://release-plz.dev/docs/config)). |
| `changelog_include = ["payg-cli"]` | On `payg` | CLI commits appear in the single changelog |
| `changelog_update = false` | On `payg-cli` | No separate per-crate changelog |
| `[changelog]` section | commit_parsers | Groups entries by type (Features, Bug Fixes, etc.). Skips `ci:`, `chore:`, `style:` commits to keep changelogs focused on user-facing changes. `protect_breaking_commits = true` ensures breaking changes are never skipped. |

#### Research Insight: `git_only = true` vs `publish = false`

The original plan used `publish = false` at the workspace level. Research revealed that `git_only = true` is the correct setting for crates not publishing to crates.io. From the [config docs](https://release-plz.dev/docs/config):

> When `git_only = true`, release-plz determines versions from git tags rather than querying the cargo registry. If no matching tag is found, the package is treated as an initial release.

This avoids the situation where release-plz tries to check crates.io for version information and gets unexpected results for unpublished crates. `git_only` and `publish` cannot both be true for the same package, but `git_only = true` implicitly skips publishing.

### 4. Create `.github/workflows/release.yml` (new file)

```yaml
name: Release

on:
  push:
    branches: [main]

# Deny all permissions by default; grant per-job
permissions: {}

jobs:
  release-plz-release:
    name: Release
    runs-on: ubuntu-latest
    # Prevent forks from running release automation
    if: github.repository == 'brettdavies/payg'
    permissions:
      contents: write
    concurrency:
      group: release-plz-release
      cancel-in-progress: false
    steps:
      - uses: actions/checkout@11bd71901bbe5b1630ceea73d27597364c9af683 # v4.2.2
        with:
          fetch-depth: 0
          persist-credentials: false
      - uses: dtolnay/rust-toolchain@stable
      - uses: release-plz/action@f708778669256143d984cce4b23592637532e040 # v0.5.127
        with:
          command: release
        env:
          GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}

  release-plz-pr:
    name: Release PR
    runs-on: ubuntu-latest
    # Prevent forks from running release automation
    if: github.repository == 'brettdavies/payg'
    permissions:
      contents: write
      pull-requests: write
    concurrency:
      group: release-plz-pr
      cancel-in-progress: false
    steps:
      - uses: actions/checkout@11bd71901bbe5b1630ceea73d27597364c9af683 # v4.2.2
        with:
          fetch-depth: 0
          persist-credentials: false
      - uses: dtolnay/rust-toolchain@stable
      - uses: release-plz/action@f708778669256143d984cce4b23592637532e040 # v0.5.127
        with:
          command: release-pr
        env:
          # NOTE: GITHUB_TOKEN cannot trigger CI on release PRs.
          # Release PRs must be manually verified before merging.
          # See: https://release-plz.dev/docs/github/token
          GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}
```

**Design notes:**

| Setting | Rationale |
|---|---|
| `release-plz/action@<SHA>` | Canonical action path ([migrated from MarcoIeni](https://github.com/release-plz/action)). SHA-pinned for supply chain security. |
| SHA pinning on all actions | Mutable tags (`@v4`, `@v0.5`) can be reassigned. The [tj-actions incident (CVE-2025-30066)](https://www.stepsecurity.io/blog/pinning-github-actions-for-enhanced-security-a-complete-guide) proved this is an active attack vector. |
| `dtolnay/rust-toolchain@stable` | Exception: not SHA-pinned because `@stable` is the intended semantic — "latest stable Rust." Pinning to a SHA would freeze the Rust version. The action author (David Tolnay) is a trusted Rust ecosystem member. |
| No `Swatinem/rust-cache@v2` | release-plz downloads a pre-built binary via cargo-binstall and never compiles your crates. The cache step wastes ~20-60s per run and writes empty entries to the 10 GB per-repo cache budget. |
| `fetch-depth: 0` | Full history for changelog generation |
| `persist-credentials: false` | Prevents token persistence in git credential store. release-plz uses the GitHub API (not git push) for all operations. |
| `if: github.repository == 'brettdavies/payg'` | Prevents fork triggers |
| Static concurrency groups | Both jobs use static keys (`release-plz-release`, `release-plz-pr`) instead of `${{ github.ref }}`. The ref is always `refs/heads/main` since the workflow only triggers on push to main. Static keys are simpler and eliminate expression injection surface. |
| Concurrency on both jobs | The release job gets `cancel-in-progress: false` to prevent duplicate tag creation from rapid pushes. The PR job gets the same to prevent skipping PR updates. |

**GITHUB_TOKEN limitation:** PRs created by the default `GITHUB_TOKEN` will not trigger `on: pull_request` workflows. This means the Release PR will not get CI checks automatically. This is acceptable because:

1. The code in the Release PR was already tested when merged to `main` via the `development → main` PR
2. The Release PR only changes `Cargo.toml` versions and `CHANGELOG.md` — no code changes
3. If CI-on-release-PRs is needed later, switch to a GitHub App token (more secure than PAT, short-lived, granular permissions)

### 5. Create `.github/dependabot.yml` (new file)

```yaml
version: 2
updates:
  - package-ecosystem: "github-actions"
    directory: "/"
    schedule:
      interval: "weekly"
```

**Why:** SHA-pinned actions need a mechanism for updates. Dependabot creates PRs with updated SHAs when new action versions are released. Without this, pinned actions would silently fall behind on security patches.

### 6. Update `.github/workflows/ci.yml`

Add `main` to `pull_request.branches` so both the `development → main` PR and any manually-created PRs to main get CI:

```yaml
pull_request:
  branches: [development, main]
```

**Why:** Currently CI only runs on PRs targeting `development`. PRs targeting `main` (including manual ones) get no checks. This is a gap regardless of release-plz.

#### Research Insight: CI Action Pinning

The existing `ci.yml` has the same mutable-tag vulnerability as the original release workflow plan. The Dependabot configuration in step 5 will automatically propose SHA-pinning updates for `ci.yml` actions too. Consider pinning `ci.yml` actions to SHAs in the same PR for consistency.

### 7. Update `CLAUDE.md`

Add a "Versioning" section after "Key Conventions":

```markdown
## Versioning

- **Scheme:** Semantic versioning (major.minor.patch)
- **Tool:** [release-plz](https://release-plz.dev/) — automated via GitHub Actions
- **Both crates version in lockstep** via `version_group` in `release-plz.toml`
- **Changelog:** Single root `CHANGELOG.md`, auto-generated from Conventional Commits
- **Release flow:** merge to `main` → release-plz opens Release PR → merge PR → git tag + GitHub release
- **Not published to crates.io yet** — `git_only = true` in `release-plz.toml`
- **Merge strategy for Release PRs:** Use standard merge commit (not squash/rebase) — squash creates a race condition in release-plz's detection
```

## Pre-Implementation: Blocking Prerequisites

These must be completed before the workflow can function:

### 1. Enable PR creation permissions (currently BLOCKING)

```sh
# Current state (verified via API):
gh api repos/brettdavies/payg/actions/permissions/workflow
# Returns: {"default_workflow_permissions":"read","can_approve_pull_request_reviews":false}

# Fix via Settings > Actions > General > Workflow permissions:
# Check "Allow GitHub Actions to create and approve pull requests"
```

Without this, the `release-plz-pr` job will fail with `Resource not accessible by integration`.

### 2. Create `v0.1.0` anchor tag (strongly recommended)

Git history analysis shows only 6 of 22 commits (27%) use Conventional Commits format. Without a tag boundary, release-plz scans all 22 commits for the changelog. The 15 freeform commits will either appear under a generic "Other" heading or be silently omitted, producing a noisy or sparse initial changelog.

Create the tag on current `main` HEAD:

```sh
git tag v0.1.0 main
git push origin v0.1.0
```

After this, release-plz will only analyze commits **after** the tag for the next release. The first automated release will be clean (likely 0.2.0, driven by the two `feat:` commits on `development` that will merge to `main`).

> **Why `v0.1.0` and not `payg-v0.1.0`?** Because `git_tag_name = "v{{ version }}"` is configured on the `payg` package, release-plz looks for tags matching `v*`. A per-crate tag format (`payg-v0.1.0`) would only match the default `{{ package }}-v{{ version }}` pattern, which we override.

## Acceptance Criteria

### Functional Requirements

- [x] Both crates inherit version from `[workspace.package]`
- [x] `cargo check --all-features` compiles with workspace inheritance
- [x] `cargo test --all-features` passes (29 pass, 13 ignored)
- [x] `release-plz.toml` configures lockstep versioning with single tag and single changelog
- [x] `.github/workflows/release.yml` runs on push to `main` with both `release` and `release-pr` jobs
- [x] All actions in `release.yml` pinned to commit SHAs (except `dtolnay/rust-toolchain@stable`)
- [x] `.github/dependabot.yml` configured for weekly action updates
- [x] CI runs on PRs targeting both `development` and `main`
- [x] CLAUDE.md documents the versioning scheme
- [ ] `v0.1.0` anchor tag exists on `main` HEAD
- [ ] `can_approve_pull_request_reviews` is enabled in repo settings

### Verification Commands

Run after implementing steps 1-3 (workspace inheritance):

```sh
# Workspace inheritance compiles
cargo check --all-features

# All tests pass (baseline: 29 pass, 13 ignored)
cargo test --all-features

# Versions resolve correctly
cargo metadata --format-version 1 --no-deps | jq '.packages[] | {name, version}'
# Expected: {"name": "payg", "version": "0.1.0"} and {"name": "payg-cli", "version": "0.1.0"}

# CLI binary reports correct version
cargo run -p payg-cli -- --version
# Expected: payg 0.1.0

# Clippy clean
cargo clippy --all-targets --all-features -- -D warnings
```

Run after pushing to `main`:

```sh
# Verify release workflow triggered
gh run list --workflow=release.yml --limit=5

# Verify Release PR was created
gh pr list --label release --state open

# Verify the anchor tag exists
git tag --list 'v*'
```

## GitHub Repo Settings Required

1. **Settings > Actions > General > Workflow permissions**: "Read and write permissions" (or keep "Read" — the workflow declares per-job permissions)
2. **Settings > Actions > General**: Check "Allow GitHub Actions to create and approve pull requests" (**currently `false` — BLOCKING**)

## First Release Expectations

With the `v0.1.0` anchor tag in place and the 6 commits from `development` merged to `main`:

| Commit | Type | Bump |
|---|---|---|
| `0afc47d` feat(v1): complete Phase 3 | `feat:` | minor |
| `d64cfa8` feat(network): add configurable network... | `feat:` | minor |
| `0720d3c` fix: resolve 8 review findings... | `fix:` | patch (subsumed) |
| `c09004a` fix(test): use unique temp HOME... | `fix:` | patch (subsumed) |
| `ca5cf98` test(on-chain): harden e2e tests... | `test:` | none |
| `28abc90` Merge remote-tracking branch... | merge | none |

**Expected first release version: 0.2.0** (two `feat:` commits produce a minor bump from 0.1.0).

The CHANGELOG will contain entries for the two features and two fixes. The `test:` commit will appear under the "Testing" group per the `commit_parsers` configuration. The merge commit has no Conventional Commits prefix and will be omitted.

## Risk Analysis

| Risk | Likelihood | Impact | Mitigation |
|---|---|---|---|
| Supply chain attack via mutable action tags | Low | Critical | All actions SHA-pinned; Dependabot for updates |
| Release PR conflicts with regular PR | Medium | Low | release-plz auto-rebases its PR on next push to main |
| Accidental 1.0.0 from `feat!:` on 0.x | Zero | High | No `!` or `BREAKING CHANGE:` markers in git history; semver bumps 0.x minor, not 1.0 |
| Fork triggers release workflow | Low | Low | `if: github.repository` guard prevents this |
| Bad release needs rollback | Low | Medium | Delete tag + GitHub Release, revert version bump commit |
| Squash merge on Release PR breaks detection | Medium | High | Documented in CLAUDE.md; all merge strategies currently enabled in repo |

### Rollback Procedure

If a bad release is created:

```sh
# Delete the git tag
git tag -d v0.2.0
git push --delete origin v0.2.0

# Delete the GitHub Release
gh release delete v0.2.0 --yes

# Revert the version bump commit on main (the merged Release PR)
git revert <merge-commit-sha>
git push origin main
```

release-plz will then recreate the Release PR from the correct state on the next push to main.

## Out of Scope (YAGNI)

- crates.io publishing (switch `git_only = true` to `publish = true` + `semver_check = true` when ready)
- Binary distribution / cargo-binstall / cargo-dist
- Per-crate changelogs
- Pre-release versions (alpha/beta/rc)
- Hotfix branching strategy (all fixes go through `development → main`)
- GitHub App token for CI-on-release-PRs
- Branch protection rules (private repo, single maintainer)
- Disabling squash merge repo-wide (document-only for now; revisit if team grows)
- SHA-pinning `ci.yml` actions (Dependabot will propose this automatically)

## References

### Internal

- Future enhancements plan (publishing): `docs/plans/2026-02-09-feat-payg-v1.1-future-enhancements-plan.md` (lines 900-925)
- CI workflow: `.github/workflows/ci.yml`
- CLI version test: `crates/payg-cli/tests/cli.rs:46-51`
- CI feature matrix solution: `docs/solutions/build-errors/rust-ci-feature-matrix-additive-gotcha.md`
- Workspace dependency conflicts solution: `docs/solutions/build-errors/rust-workspace-dependency-conflicts-and-feature-flags.md`

### External

- [release-plz configuration reference](https://release-plz.dev/docs/config)
- [release-plz GitHub Action quickstart](https://release-plz.dev/docs/github/quickstart)
- [release-plz/action repository](https://github.com/release-plz/action) — v0.5.127 SHA: `f708778669256143d984cce4b23592637532e040`
- [release-plz token documentation](https://release-plz.dev/docs/github/token)
- [GitHub: Actions token cannot trigger workflows](https://github.com/orgs/community/discussions/25602)
- [Orhun: Fully Automated Releases for Rust](https://blog.orhun.dev/automated-rust-releases/)
- [StepSecurity: Pinning GitHub Actions for Enhanced Security](https://www.stepsecurity.io/blog/pinning-github-actions-for-enhanced-security-a-complete-guide)
- [Quantinuum/hugr release-plz.toml](https://github.com/Quantinuum/hugr/blob/main/release-plz.toml) — real-world workspace config example
