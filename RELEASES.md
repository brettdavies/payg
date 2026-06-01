# Releasing `payg`

Operational runbook. Rationale lives in [`RELEASES-RATIONALE.md`](./RELEASES-RATIONALE.md). Pre-tag gate checklist lives
in [`RELEASES-PREFLIGHT.md`](./RELEASES-PREFLIGHT.md).

```text
feature branch → PR to dev (squash merge)
              → dev accumulates commits
              → release-plz opens "Release v<version>" PR from main side
              → merge Release PR (merge commit, NOT squash)
              → tag push triggers GitHub Release
```

Direct commits to `main` are blocked by the pre-commit hook and the `Protect main` ruleset. `dev` permits direct commits
for engineering docs only (see [§ Dev-direct exception](#dev-direct-exception)); all code changes flow through PRs.

## Branches

| Branch                                 | Role                                                  | Lifetime                                    | Protection                           |
| -------------------------------------- | ----------------------------------------------------- | ------------------------------------------- | ------------------------------------ |
| `main`                                 | Production. Release commits + dev integration merges. | Forever.                                    | `.github/rulesets/protect-main.json` |
| `dev`                                  | Integration. All feature PRs land here.               | Forever. Never delete.                      | `.github/rulesets/protect-dev.json`  |
| `feat/*`, `fix/*`, `chore/*`, `docs/*` | Feature work.                                         | One PR's worth. Auto-deleted on merge.      | None. Squash into dev freely.        |
| `release-plz-*`                        | Auto-opened by release-plz with version bump.         | One release's worth. Auto-deleted on merge. | None.                                |

→ Rationale: [`RELEASES-RATIONALE.md` § Branching model](./RELEASES-RATIONALE.md#branching-model).

## Daily development (feature → dev)

```bash
git checkout dev && git pull
git checkout -b feat/short-description
# ... work ...
git push -u origin feat/short-description
gh pr create --base dev --title "feat(scope): what changed"
# CI passes → squash-merge (PR_TITLE becomes the commit subject, PR_BODY the body)
```

- **Commit style**: [Conventional Commits](https://www.conventionalcommits.org/). The `commit_parsers` in
  `release-plz.toml` categorize them into changelog groups; `ci`/`chore`/`style` get skipped.
- **Prefer `feat` / `fix`** over `chore` for any user-observable change. See
  [`RELEASES-RATIONALE.md` § Why `feat`/`fix` are preferred over `chore`](./RELEASES-RATIONALE.md#why-featfix-are-preferred-over-chore).
- **PR body**: follow `.github/pull_request_template.md`. Brief Summary, Type of Change checkbox, Testing checkboxes. No
  verification narration in the body.
- **No AI attribution** in commits or PR bodies.

### Dev-direct exception

Engineering docs that live only on `dev` may be committed directly to `dev` without a feature branch:

- `docs/brainstorms/`, `docs/ideation/`, `docs/plans/`, `docs/research/`, `docs/reviews/`, `docs/solutions/`
- Anything under `.context/`

The standard feature → PR → squash-merge flow remains required for everything else, including consumer-facing markdown
(`README.md`, `AGENTS.md`, `CONTRIBUTING.md`, `CHANGELOG.md`, in-repo runbooks like this one).

PAYG does **not** currently enforce a `guard-main-docs` workflow; the convention is honored manually. If guarded-doc
leakage onto `main` becomes a real problem, see
[`docs/plans/2026-06-01-chore-migrate-to-reusable-workflows-plan.md`](./docs/plans/2026-06-01-chore-migrate-to-reusable-workflows-plan.md)
for the in-flight migration that would add the guard.

## Releasing dev to main

PAYG uses `release-plz` to automate the release-PR cut. The flow is asymmetric:

- Merge commits from `dev` into `main` via a normal PR. The merge strategy is **a true merge commit, not squash**. See
  [`RELEASES-RATIONALE.md` § Why merge commits to main, not squash](./RELEASES-RATIONALE.md#why-merge-commits-to-main-not-squash).
- After the merge lands on `main`, `release.yml` runs and `release-plz release-pr` opens (or updates) a `release-plz-*`
  branch with:
- `version` bumped in both crates (lockstep via `version_group = "payg"`)
- `Cargo.lock` updated
- `CHANGELOG.md` regenerated from Conventional Commits
- Review the Release PR's diff. The `## Changelog` content is the source of truth for the GitHub Release notes.
- Merge the Release PR. The push triggers the `release-plz-release` job, which tags `v<version>` and creates the GitHub
  Release.

```bash
# Promote dev to main (manual PR)
git checkout main && git pull
git merge --no-ff origin/dev -m "merge: integrate dev for vX.Y.Z"
# Open the integration PR via the GitHub UI or:
gh pr create --base main --head dev --title "merge: integrate dev" --body-file /tmp/integration-body.md

# After the merge lands, release-plz opens the Release PR automatically.
# Review on GitHub, then merge it to publish.
```

→ Rationale (release-plz model, asymmetric squash-vs-merge, lockstep versioning):
[`RELEASES-RATIONALE.md` § Release model](./RELEASES-RATIONALE.md#release-model).

### Pre-tag gate

Before merging the Release PR, walk the pre-flight checklist: [`RELEASES-PREFLIGHT.md`](./RELEASES-PREFLIGHT.md). Each
item gates the cut and CI structurally cannot enforce all of them.

## Publishing (deferred)

PAYG runs `git_only = true` in `release-plz.toml`. The Release PR + tag + GitHub Release ships, but **nothing is pushed
to crates.io and no Homebrew formula is updated**. These channels are deferred to the launch sprint.

When ready to enable crates.io publish:

1. Verify the email on the publishing account (`https://crates.io/settings/profile`).
2. Run `cargo publish -p payg` and `cargo publish -p payg-cli` from `main` at the release tag, with a
   `CARGO_REGISTRY_TOKEN` set locally.
3. Configure Trusted Publishing at `https://crates.io/settings/tokens/trusted-publishing` for both crates.
4. Flip `git_only = true` → `git_only = false` in `release-plz.toml` (or remove the key — `false` is the default).
   Commit and ship through a release.

Homebrew is a separate decision; the current expectation is the brettdavies homebrew-tap dispatch pattern, which depends
on the broader workflow migration tracked in `docs/plans/2026-06-01-chore-migrate-to-reusable-workflows-plan.md`.

## Branch protection

Two rulesets live under `.github/rulesets/` and are applied to the repo via the GitHub API:

- `protect-main.json` — required signatures, linear history not enforced (merge commits permitted from dev), required
  status checks (`test ()`, `test (--no-default-features --features eth)`, `test (--all-features)`, `lint`, `msrv`,
  `deny`, `doc`), creation / deletion blocked, non-fast-forward blocked.
- `protect-dev.json` — required signatures, deletion blocked, non-fast-forward blocked.

### Applying changes

```bash
# First apply (creating a ruleset):
gh api -X POST repos/brettdavies/payg/rulesets --input .github/rulesets/protect-dev.json

# Subsequent updates (replace by ID — find via `gh api repos/brettdavies/payg/rulesets`):
gh api -X PUT repos/brettdavies/payg/rulesets/<id> --input .github/rulesets/protect-main.json
```

→ Status-check context strings (inline vs reusable, why migration is blocked):
[`RELEASES-RATIONALE.md` § Status-check context strings](./RELEASES-RATIONALE.md#status-check-context-strings).

## Required secrets

`GITHUB_TOKEN` is the only secret consumed by `release.yml`; it is auto-provisioned. CI (`ci.yml`) only needs `contents:
read` and uses no extra secrets.

A `CI_RELEASE_TOKEN` (fine-grained PAT) is provisioned for future workflow needs (Homebrew dispatch, crates.io OIDC
bootstrap) but is not consumed by any current job. See
`docs/solutions/architecture-patterns/github-pat-consolidation-across-repos-20260319.md` in the solutions repo for the
consolidation rationale.

## Related docs

- [`RELEASES-PREFLIGHT.md`](./RELEASES-PREFLIGHT.md) — pre-tag verification gate.
- [`RELEASES-RATIONALE.md`](./RELEASES-RATIONALE.md) — the WHY behind the release model.
- [`AGENTS.md`](./AGENTS.md) — project conventions, environment variables, build commands.
- [`.github/pull_request_template.md`](./.github/pull_request_template.md) — PR body structure.
- [`release-plz.toml`](./release-plz.toml) — release-plz configuration (single source of truth for changelog grouping).
