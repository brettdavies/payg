# Releases rationale

Companion to [`RELEASES.md`](./RELEASES.md). `RELEASES.md` is the runbook (commands, paths, decision tables). This file
holds the WHY behind those rules: branching model, release model, CHANGELOG generation, deferred items,
branch-protection pitfalls.

Read this when:

- A rule in `RELEASES.md` doesn't make sense and you're tempted to change it.
- A new contributor asks "why do we do X this way".
- You're adding a new release-flow rule and need to know where it fits the existing model.

## Branching model

### Forever `dev` and forever `main`

Both `dev` and `main` are forever branches. Feature branches squash-merge into `dev`. `main` receives merge commits from
`dev` via PR. The repo's `deleteBranchOnMerge: true` setting safely deletes the ephemeral feature and `release-plz-*`
branches because neither `dev` nor `main` is ever a PR *head*.

The asymmetry is deliberate: feature branches squash-merge into `dev` (clean, one-commit-per-feature history); `dev`
*merges* into `main` (preserves the per-feature commits in `main`'s history so release-plz can attribute each line in
`CHANGELOG.md` back to its originating commit).

### Why merge commits to main, not squash

PAYG enforces "Standard merge commit to main (not squash/rebase)" via the branch ruleset, intentionally diverging from
the brettdavies default (squash-only). Reason captured in [`AGENTS.md`](./AGENTS.md): squash-merging `dev` → `main`
creates a **race condition in release-plz** where unreviewed commits can slip into a release.

The race: release-plz opens its Release PR off the current `main`. If a squash-merge of `dev` rewrites the per-commit
history at merge time, commits release-plz inspected during PR generation may differ from what actually lands. A true
merge commit preserves the dev-side SHAs verbatim, so release-plz's view and the final state agree.

This is why the github-repo-setup standard's "squash-only" recommendation is overridden in this repo's ruleset.

### No `release/*` cherry-pick branches

The agentnative-cli flow uses ephemeral `release/v<version>` branches to cherry-pick a curated subset of `dev` commits
onto `main`. PAYG does not. The reasons:

- PAYG ships the entire `dev` integration as-is; there is no "skip these docs, ship those features" curation step.
- Engineering docs (`docs/plans/`, `docs/brainstorms/`, etc.) currently *do* reach `main` because no `guard-main-docs`
  workflow blocks them. This is the gap captured in
  [`docs/plans/2026-06-01-chore-migrate-to-reusable-workflows-plan.md`](./docs/plans/2026-06-01-chore-migrate-to-reusable-workflows-plan.md).
- release-plz expects to operate against a linear-ish `main` history; the `release/*` cherry-pick model competes with
  it.

If PAYG ever needs the `release/*` pattern (to keep planning docs off `main`), it would require migrating off
release-plz to the brettdavies reusable workflows. That decision is documented but deferred.

## Release model

### Why release-plz

release-plz automates the parts of a release that are mechanical and easy to get wrong:

- Version bumps in `Cargo.toml` files (both crates lockstep via `version_group = "payg"`).
- `Cargo.lock` regeneration.
- `CHANGELOG.md` generation from Conventional Commits.
- Git tag creation (`v{{ version }}` per `git_tag_name`).
- GitHub Release creation with the new changelog section.

The cost is a release pipeline that *only* knows release-plz's vocabulary. The standard brettdavies reusable workflows
(`rust-release.yml`) implement a different model (tag-push triggers cross-compile + draft-then-finalize + Homebrew
dispatch). The two are not variants of the same pipeline; they are different pipelines.

PAYG picked release-plz before the brettdavies standard solidified. The trade-off is reviewed in
`docs/plans/2026-06-01-chore-migrate-to-reusable-workflows-plan.md`.

### Why `git_only = true`

`release-plz.toml` sets `git_only = true`, which disables crates.io publishing. PAYG is not yet on crates.io. The
reasons:

- The CLI surface is still settling (commands, flags, env var names).
- The library API is still settling (whether to keep the `Backend::X402` / `Backend::Eth` enum, or move to a trait when
  a third backend appears).
- A premature publish locks the crate name and ships a SemVer commitment we are not yet ready to maintain.

Flipping `git_only = false` is a deliberate launch-sprint decision, not a side-effect of any code change.

### Lockstep versioning

Both `payg` (library) and `payg-cli` (binary) version in lockstep via `version_group = "payg"` in `release-plz.toml`.
The reasons:

- The CLI is the only consumer of the library at v1; their bug surfaces are entangled.
- Independent versioning would force consumers (Rust API users *and* end users running the CLI) to mentally map two
  version numbers.
- The combined `CHANGELOG.md` (`changelog_include = ["payg-cli"]`) tells a single story per release.

`payg-cli`'s `git_tag_enable = false` and `git_release_enable = false` prevent release-plz from creating duplicate tags
/ releases for the CLI crate; `payg` owns the tag for the lockstep pair.

### Why `changelog_update = false` on `payg-cli`

A single root `CHANGELOG.md` is the source of truth. Disabling per-crate changelog files on `payg-cli` prevents
release-plz from auto-generating a second `crates/payg-cli/CHANGELOG.md` that would silently drift from the root one.
The root file aggregates both crates' commits via `changelog_include`.

## CHANGELOG generation

### Generated, never hand-written

release-plz regenerates `CHANGELOG.md` from Conventional Commits each time it opens or updates the Release PR. The
single source of truth is the commit subject prefix:

| Prefix        | Group         |
| ------------- | ------------- |
| `feat(...)`   | Features      |
| `fix(...)`    | Bug Fixes     |
| `docs(...)`   | Documentation |
| `perf(...)`   | Performance   |
| `refactor(`   | Refactoring   |
| `test(...)`   | Testing       |
| `revert(...)` | Reverted      |
| `ci(...)`     | *(skipped)*   |
| `chore(...)`  | *(skipped)*   |
| `style(...)`  | *(skipped)*   |

To fix a wrong `CHANGELOG.md` entry, fix the input: edit the commit subject (rare, and only on the integration merge
before it hits `main`), then close + reopen the Release PR so release-plz regenerates. Do **not** edit `CHANGELOG.md`
directly; the next release-plz run will overwrite it.

### Why `cliff.toml`-style skips apply

The `commit_parsers` block in `release-plz.toml` skips `ci`, `chore`, and `style`. These commit types do not produce
user-facing content. If a cherry-picked PR has user-facing content but its commit subject starts with one of those
types, its bullets get silently dropped.

### Why `feat`/`fix` are preferred over `chore`

`release-plz.toml` parses commit subjects in order. The first matching parser wins. `chore` matches anything starting
with `^chore`, full stop. Mistyping a user-facing change as `chore` silently strips it from `CHANGELOG.md`. Prefer
`feat` / `fix` when the change has any user-observable effect (config defaults, env vars, default behaviors, new
commands, new flags).

This is identical to the rationale in the global `CLAUDE.md` § Commits & PRs; the rule applies regardless of which
release tool generates the changelog.

## Status-check context strings

The `required_status_checks[].context` strings in `protect-main.json` MUST match exactly what GitHub publishes for each
check. Today they are inline-job names with no prefix:

```text
test ()
test (--no-default-features --features eth)
test (--all-features)
lint
msrv
deny
doc
```

The `test (...)` rows are matrix-job published names. The empty `()` is GitHub's published name for the first matrix
axis value (`features: ""`). Don't reformat these.

If PAYG migrates `ci.yml` from inline jobs to a reusable-workflow caller (per
`docs/plans/2026-06-01-chore-migrate-to-reusable-workflows-plan.md`), the published context shape will change to
`<caller-job-id> / <reusable-job-id-or-name>`. The ruleset must be updated in lockstep with the migration or every PR
will get stuck waiting on a context that will never report. Confirm the real contexts after a first CI run with:

```bash
gh api repos/brettdavies/payg/commits/<sha>/check-runs --jq '.check_runs[].name'
```

See
[`docs/solutions/integration-issues/github-status-check-context-inline-vs-reusable-2026-04-14.md`](./docs/solutions/integration-issues/github-status-check-context-inline-vs-reusable-2026-04-14.md)
in the solutions repo for the full writeup of the inline-vs-reusable context pitfall.

## Deferred items

These are deliberate omissions from the current release model, captured here so they don't get rediscovered as
"missing":

| Item                            | Why deferred                                                                                                                     | Tracking                                                            |
| ------------------------------- | -------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------- |
| crates.io publish               | API and CLI surface still settling; premature name claim                                                                         | `release-plz.toml` `git_only = true`                                |
| Homebrew formula                | Depends on cross-compile + dispatch chain that depends on workflow migration                                                     | `docs/plans/2026-06-01-chore-migrate-to-reusable-workflows-plan.md` |
| Cross-compiled release binaries | Same as Homebrew; tied to the reusable-workflow migration                                                                        | Same plan as above                                                  |
| `guard-main-docs.yml` workflow  | PAYG ships planning docs on `main` today; adding the guard requires the `release/*` cherry-pick pattern                          | Same plan as above                                                  |
| Trusted Publishing (OIDC)       | Only relevant once crates.io publish is enabled                                                                                  | Will be set up during the launch-sprint publish                     |
| Vale / LanguageTool prose scrub | The brettdavies prose-check stack is anchored in `agentnative-spec`; PAYG doesn't vendor it yet. PR bodies are reviewed by hand. | No active tracking; revisit if PR body quality drifts               |

## Branch protection

### Why rulesets live in-repo

Committing the JSON alongside code means ruleset changes land via the same review process as workflow changes. A
`chore(ci): tighten protect-main` change goes through `dev` → `main` like anything else. The `gh api -X PUT` step that
pushes the ruleset to GitHub is a manual side-effect run by a maintainer after merge; there is no auto-apply.

### Why `dev` is protected more lightly than `main`

The `protect-dev.json` ruleset blocks deletion, blocks force-push, and requires signed commits, but does not require a
PR. Engineering docs land directly. The PR-only norm for code changes is honored by convention plus the pre-commit hook
(which blocks direct commits to `main`, not `dev`).

If a contributor ever needs to bypass-push to `dev` to recover from a tangled state, the lighter protection makes that
possible without involving an admin bypass. `main` has no such escape hatch by design.

## Related docs

- [`RELEASES.md`](./RELEASES.md): operational runbook (commands, paths, decision tables).
- [`RELEASES-PREFLIGHT.md`](./RELEASES-PREFLIGHT.md): pre-tag gate checklist.
- [`AGENTS.md`](./AGENTS.md): project conventions, environment variables, feature flags.
- [`release-plz.toml`](./release-plz.toml): release-plz configuration (single source of truth for changelog grouping).
-
  [`docs/plans/2026-06-01-chore-migrate-to-reusable-workflows-plan.md`](./docs/plans/2026-06-01-chore-migrate-to-reusable-workflows-plan.md):
  workflow-migration gap and the items it unblocks.
