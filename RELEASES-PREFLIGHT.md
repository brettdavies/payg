# Pre-release verification: `payg`

Operational pre-flight checklist. Runs **before** merging the release-plz Release PR in
[`RELEASES.md` § Releasing dev to main](./RELEASES.md#releasing-dev-to-main). Each box is an explicit go/no-go. If any
item is unchecked or red, hold the release.

CI (fmt, clippy, test across the feature matrix, MSRV check, doc, cargo-deny) catches mechanical regressions inside this
repo. This checklist covers what CI structurally can't:

- Behavior against real RPC endpoints and the x402 facilitator (CI mocks both).
- Backend-feature interaction: `x402`-only, `eth`-only, and `--all-features` configurations need explicit eyes — the
  matrix runs them but doesn't sanity-check their public APIs against each other.
- Wallet-loading paths that depend on real keyfiles or live env vars.
- Cross-network sanity: defaults align between `base-sepolia` (testnet) and `base` (mainnet, real funds).

## Establish the surface

Everything below assumes you know what's changing. Run this first.

```bash
LAST_TAG=$(git tag --sort=-version:refname | head -n 1)
git log "$LAST_TAG..main" --oneline                              # commits going out
git diff "$LAST_TAG..main" --stat                                # file-level scope
git diff "$LAST_TAG..main" -- crates/payg/src/                   # library API surface
git diff "$LAST_TAG..main" -- crates/payg-cli/src/               # CLI surface
git log "$LAST_TAG..main" --grep '^[a-z]\+!:' --oneline          # Conventional-Commits breaking markers
```

Every `!:` commit drives the major-version decision and gets called out in the Release PR body.

## Checklist

### Public-API surface

- [ ] `crates/payg/src/lib.rs` — every newly exported symbol is intentional. Library consumers depend on the public
  surface; accidental `pub` additions become a SemVer commitment.
- [ ] `crates/payg/src/error.rs` — any new `PaygError` variant is reviewed for whether it leaks a feature-flagged type
  (e.g., `reqwest::Error` would re-introduce the feature-flag leak avoided per [`AGENTS.md`](./AGENTS.md) § Key
  Conventions).
- [ ] No new `pub use` re-exports from feature-gated modules without `#[cfg(feature = ...)]`.

### Backend-feature matrix

The CI matrix runs three configurations. Confirm each works end-to-end, not just compiles:

- [ ] **Default (`x402` only):** `cargo test -p payg` and `cargo test -p payg-cli` both pass. `--no-default-features`
  without `--features eth` fails with the `compile_error!` guard (regression check for the no-backend safety net).
- [ ] **`eth` only:** `cargo test -p payg --no-default-features --features eth`. The `Backend::Eth` arm of every `match`
  reachable from the CLI is exercised by at least one test.
- [ ] **`--all-features`:** both backend arms compile and `Backend::X402(...)` vs `Backend::Eth(...)` dispatch cleanly
  without surprising type-coercion warnings.

### Network and chain configuration

- [ ] `PAYG_NETWORK` defaults still point at `base-sepolia` (testnet). Anyone running `payg charge` without an explicit
  network does **not** hit mainnet by accident.
- [ ] `PAYG_MAX_CHARGE` default ceiling is still in place and is documented in the README.
- [ ] If the x402 facilitator URL default has been changed, the new default is a known-good upstream — not a community
  fork URL captured during testing.
- [ ] If a new network has been added (mainnet, other L2), the chain ID, default RPC, and facilitator URL combination is
  end-to-end verified in `cargo test -- --ignored` against the actual chain (or explicitly deferred and called out in
  the Release PR).

### Wallet and secret hygiene

- [ ] No private keys, mnemonics, or test wallet seeds in any test fixture, doctest, or example. Run `git diff
  "$LAST_TAG..main" -- '*.rs' '*.toml' '*.md'` and scan for hex strings ≥40 chars.
- [ ] `PAYG_PRIVATE_KEY` and `PAYG_KEY_PASSWORD` continue to be documented as env-only inputs. No accidental config-file
  path was added that would tempt users to commit secrets.
- [ ] Keyfile decryption still uses scrypt (no silent KDF downgrade). Spot-check `crates/payg/src/wallet.rs` if
  `eth-keystore` was bumped.

### Release mechanics sanity

These items duplicate steps elsewhere deliberately: easy to skip, expensive to recover from.

- [ ] release-plz Release PR opened correctly: both crate versions bumped in lockstep (`version_group = "payg"`
  working), `Cargo.lock` updated, `CHANGELOG.md` regenerated.
- [ ] `CHANGELOG.md` content matches the commits since `$LAST_TAG`. No user-facing change is missing because a commit
  was typed as `chore` instead of `feat`/`fix`. See
  [`RELEASES-RATIONALE.md` § Why `feat`/`fix` are preferred over `chore`](./RELEASES-RATIONALE.md#why-featfix-are-preferred-over-chore).
- [ ] `Cargo.toml` MSRV (`rust-version = "1.88.0"`) is unchanged, or if bumped, the bump is intentional and called out
  in the Release PR.
- [ ] `rand` is still pinned to `0.8` (alloy-rs compatibility). If a transitive bump tried to advance it, the lockfile
  must still resolve to a `0.8.x` line.
- [ ] No unmerged dependency advisories from `cargo deny check advisories`.

### Cross-repo / external considerations

- [ ] If `release-plz.toml` settings changed (e.g., toggling `git_only`), confirm the change is intentional. Flipping
  `git_only = false` triggers a crates.io publish on the next tag, which is a one-way door unless yanked.
- [ ] If publishing to crates.io has been enabled, both `payg` and `payg-cli` names are still available / owned. Check
  `cargo search payg` and confirm the publishing account has the crate.
- [ ] No unmerged Dependabot security advisories in the PR queue.

### Post-tag verification

Run immediately after the Release PR merges and `release.yml` runs:

- [ ] `release.yml` green end-to-end. `gh run watch <id> --exit-status`, then verify with `gh run view <id> --json
  conclusion --jq .conclusion` (the watcher exit code alone is not authoritative — see global CLAUDE.md § CI after
  push).
- [ ] GitHub Release exists at `v<version>` and the release notes carry the curated `CHANGELOG.md` section.
- [ ] If crates.io publish is enabled: the new version appears at `https://crates.io/crates/payg` and
  `https://crates.io/crates/payg-cli`. `cargo install payg-cli --version <new>` from a clean environment resolves and
  runs.
- [ ] Manual smoke from a clean checkout at the new tag: `cargo run -p payg-cli -- --version` prints the new tag value.

## Related docs

- [`RELEASES.md`](./RELEASES.md) — operational runbook this checklist gates.
- [`RELEASES-RATIONALE.md`](./RELEASES-RATIONALE.md) — release-flow rationale.
- [`AGENTS.md`](./AGENTS.md) — project conventions, environment variables, feature flags.
