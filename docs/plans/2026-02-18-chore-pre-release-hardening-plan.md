---
title: "chore: Pre-release hardening"
type: chore
status: completed
date: 2026-02-18
deepened: 2026-02-18
brainstorm: docs/brainstorms/2026-02-18-pre-release-hardening-brainstorm.md
---

# Pre-Release Hardening

## Enhancement Summary

**Deepened on:** 2026-02-18
**Agents used:** Security Sentinel, Code Simplicity Reviewer, Architecture Strategist, Pattern Recognition Specialist, Best Practices Researcher, Framework Docs Researcher

### Critical Fixes Discovered

1. **deny.toml uses removed fields** --- `vulnerability`, `notice`, and `severity-threshold` were removed in cargo-deny 0.16+ and cause hard errors. Replaced with scope-based `unmaintained`/`unsound` fields.
2. **Missing `ring` license clarification** --- `ring` has non-standard license metadata. Without a `[[licenses.clarify]]` block, cargo-deny will fail on license checks.
3. **cargo-deny-action requires `rust-version: "1.88.0"`** --- the Docker image ships Rust 1.71.0, but the payg workspace requires 1.88.0.

### Key Improvements

1. Rewrote `deny.toml` based on alloy-rs and Foundry reference configs, with stricter supply chain settings
2. Expanded SECURITY.md scope with 6 additional items identified by security review
3. Upgraded issue templates from markdown (.md) to YAML forms (.yml) for structured input validation
4. Dropped `keywords`/`categories` metadata (YAGNI --- crates.io-only fields, `git_only = true`)
5. Dropped `eip712_extra` test (4-line field copy, no logic to verify)
6. Trimmed `format_token_amount` from 6 to 4 tests (function is parameterized by decimals, not branched)

### New Concerns Identified (Deferred)

- `charge_raw()` is a public API that bypasses the safety ceiling --- consider restricting to `pub(crate)` post-release
- Passwords in `wallet_helper.rs` and `init.rs` are not wrapped in `Zeroizing<String>` (private keys are, passwords are not)
- `protect-development.json` has no required status checks (pre-existing gap)

---

## Overview

Harden the repo before the initial v0.1.0 git release. The codebase is functionally complete --- this work covers security posture, CI supply chain integrity, documentation accuracy, test coverage for pure logic, and contributor experience. Not publishing to crates.io.

## Problem Statement

1. README license section says "MIT" but the repo is dual-licensed MIT OR Apache-2.0
2. No anchor git tag --- release-plz will dump all history into the first changelog
3. No dependency vulnerability scanning in CI (485 dependencies)
4. No vulnerability disclosure policy (SECURITY.md)
5. CI workflow uses mutable action tags while release workflow SHA-pins
6. Dependabot only monitors GitHub Actions, not Cargo dependencies
7. Crate metadata missing `repository` and `homepage` fields
8. `check_safety_ceiling()` has a known P1 bug (todo 025) and zero unit tests
9. `format_token_amount` has no unit tests
10. 31 pending todos, many referencing issues already fixed in code
11. No GitHub issue templates

## Implementation Phases

### Phase 1: Fixes and Anchor Tag

These unblock release-plz and fix a legal inaccuracy.

#### 1a. Fix README license section

**File:** `README.md:268-270`

Replace:

```markdown
## License

MIT
```

With:

```markdown
## License

Dual-licensed under [MIT](LICENSE-MIT) or [Apache-2.0](LICENSE-APACHE), at your option.
```

#### 1b. Create anchor git tag

Create `v0.1.0` tag on `main` at the current HEAD to establish the changelog boundary for release-plz.

```bash
git tag v0.1.0 main
git push origin v0.1.0
```

**Reference:** `docs/solutions/integration-issues/release-plz-rust-workspace-setup.md` --- Gotcha #6: anchor tag must exist before release-plz runs.

**Important:** This must be done on `main` AFTER all other changes in this plan are merged. The tag marks the release-ready baseline. Do this as the very last step.

---

### Phase 2: CI Hardening

#### 2a. SHA-pin CI workflow actions

**File:** `.github/workflows/ci.yml`

Replace mutable tags with immutable SHA pins. Use the same `actions/checkout` SHA as `release.yml` for consistency. Dependabot will keep SHAs current.

| Action | Current | Target |
|--------|---------|--------|
| `actions/checkout@v4` | Mutable tag | `actions/checkout@11bd71901bbe5b1630ceea73d27597364c9af683 # v4.2.2` |
| `Swatinem/rust-cache@v2` | Mutable tag | `Swatinem/rust-cache@401aff9a7a08acb9d27b64936a90db81024cff97 # v2.8.2` |
| `dtolnay/rust-toolchain@stable` | Keep as-is | Intentionally unpinned --- `@stable` is a rolling branch by design |
| `dtolnay/rust-toolchain@master` | Keep as-is | Intentionally unpinned --- same reason; MSRV job needs latest master to resolve `1.88.0` |

Three occurrences of `actions/checkout@v4` (lines 23, 34, 45) and three of `Swatinem/rust-cache@v2` (lines 25, 38, 49).

**Reference:** CVE-2025-30066 (compromised `tj-actions/changed-files` via mutable tags). Documented in `docs/solutions/integration-issues/release-plz-rust-workspace-setup.md`.

> **Research Insight:** This introduces a convention change --- currently CI uses tag refs while release uses SHA pins. Document this in a commit message or CLAUDE.md so the convention is explicit. After this change, both workflows use SHA pins for `actions/checkout` and `Swatinem/rust-cache`, with `dtolnay/rust-toolchain` intentionally unpinned in both.

#### 2b. Add cargo-deny to CI

**Decision:** Use `cargo-deny` over `cargo-audit` --- it covers vulnerabilities (same RustSec DB) PLUS license compliance, banned crates, and duplicate detection. License checking is valuable for a dual-licensed project.

**New file:** `deny.toml` (root)

```toml
[advisories]
yanked = "deny"
unmaintained = "warn"
unsound = "warn"
ignore = []

[licenses]
version = 2
unlicensed = "deny"
confidence-threshold = 0.8
allow = [
    "MIT",
    "Apache-2.0",
    "Apache-2.0 WITH LLVM-exception",
    "BSD-2-Clause",
    "BSD-3-Clause",
    "0BSD",
    "ISC",
    "Unicode-3.0",
    "Unicode-DFS-2016",
    "Zlib",
    "BSL-1.0",
    "MPL-2.0",
    "Unlicense",
    "CC0-1.0",
    "OpenSSL",
]
exceptions = [
    { allow = ["CC0-1.0"], crate = "tiny-keccak" },
    { allow = ["CC0-1.0"], crate = "secp256k1" },
    { allow = ["CC0-1.0"], crate = "secp256k1-sys" },
]

[[licenses.clarify]]
crate = "ring"
expression = "LicenseRef-ring"
license-files = [{ path = "LICENSE", hash = 0xbd0eed23 }]

[licenses.private]
ignore = true

[bans]
multiple-versions = "warn"
wildcards = "deny"
highlight = "all"
deny = [
    { crate = "openssl", reason = "use rustls instead" },
    { crate = "openssl-sys", reason = "use rustls instead" },
]

[sources]
unknown-registry = "deny"
unknown-git = "deny"
allow-registry = ["https://github.com/rust-lang/crates.io-index"]
allow-git = []
```

> **Research Insights (cargo-deny configuration):**
>
> **Breaking change in cargo-deny 0.16+:** The fields `vulnerability`, `notice`, and `severity-threshold` in `[advisories]` were REMOVED and cause hard errors if present. Security vulnerabilities are now always checked at deny level. Use scope-based fields (`unmaintained`, `unsound`) instead.
>
> **`ring` license clarification:** The `ring` crate has non-standard license metadata (not valid SPDX). Without the `[[licenses.clarify]]` block with the correct file hash, cargo-deny will fail. The hash `0xbd0eed23` is from the alloy-rs reference config and must be verified against the actual `ring` version in `Cargo.lock` (currently v0.17.14).
>
> **`exceptions` vs `allow`:** CC0-1.0 is used by `tiny-keccak`, `secp256k1`, and `secp256k1-sys` (alloy-rs transitive deps). Using per-crate `exceptions` instead of global `allow` keeps the license policy tight --- new CC0-1.0 crates will be flagged for review.
>
> **Supply chain strictness:** `wildcards = "deny"` prevents `*` version requirements in transitive deps. `unknown-registry = "deny"` and `unknown-git = "deny"` hard-fail on non-crates.io sources. These are appropriate for a project handling private keys and financial transactions.
>
> **OpenSSL ban:** The project uses `rustls` via reqwest's default features. Banning `openssl` and `openssl-sys` prevents accidental native TLS linkage from future dependency additions.
>
> **`version = 2`:** Required for modern deny.toml format.
>
> **`[licenses.private] ignore = true`:** Skips license checks for workspace members marked `publish = false`, which is correct for a `git_only` project.
>
> **Reference configs:** Based on [alloy-rs deny.toml](https://github.com/alloy-rs/alloy/blob/main/deny.toml) and [Foundry deny.toml](https://github.com/foundry-rs/foundry/blob/master/deny.toml).

**File:** `.github/workflows/ci.yml` --- add new job:

```yaml
  deny:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@11bd71901bbe5b1630ceea73d27597364c9af683 # v4.2.2
      - uses: EmbarkStudios/cargo-deny-action@44db170f6a7d12a6e90340e9e0fca1f650d34b14 # v2.0.15
        with:
          rust-version: "1.88.0"
          command: check
          arguments: --all-features
```

> **Research Insight:** The cargo-deny-action Docker image ships Rust 1.71.0. Since the payg workspace requires `rust-version = "1.88.0"`, the `rust-version` input is mandatory or the action will fail. The `--all-features` argument ensures both `x402` and `eth` backend dependencies are scanned.

No need for `rust-toolchain` or `rust-cache` --- the cargo-deny action bundles its own binary.

**Update branch ruleset:** Add `deny` to the required status checks in `.github/rulesets/protect-main.json`. Stage the rollout: merge CI changes first, verify the deny job passes on at least one PR cycle, then update the ruleset.

#### 2c. Add Cargo ecosystem to Dependabot

**File:** `.github/dependabot.yml`

```yaml
version: 2
updates:
  - package-ecosystem: "github-actions"
    directory: "/"
    schedule:
      interval: "weekly"
  - package-ecosystem: "cargo"
    directory: "/"
    schedule:
      interval: "weekly"
```

> **Research Insight (pattern consistency):** The existing Dependabot config is minimal --- no groups, labels, commit-message prefixes, or reviewers. Match this style for the new `cargo` entry. If enrichments are added later, apply them to BOTH entries for consistency. Dependabot supports Cargo workspaces via `directory: "/"` pointing to the workspace root.

---

### Phase 3: Documentation and Metadata

#### 3a. Add SECURITY.md

**New file:** `SECURITY.md` (root)

Contents:

- **Supported versions:** Table showing v0.1.x as supported
- **Reporting vulnerabilities:** Private disclosure via GitHub Security Advisories ("Report a vulnerability" button, preferred) or email. Include repo URL.
- **Response timeline:** Acknowledge within 48 hours, patch within 7 days for critical
- **Scope:** (expanded from original plan based on security review)
  - Private key handling and keyfile confidentiality (permissions, symlinks, path traversal)
  - Wallet operations and key loading
  - Payment execution and receipt integrity
  - Safety ceiling bypass or circumvention
  - Configuration injection via `payg.toml` or environment variables
  - Facilitator response integrity (fake `tx_hash`)
  - Private key exposure via environment variables
  - Network mismatch leading to unintended mainnet transactions
  - URL validation bypass
  - Dependency vulnerabilities
- **Out of scope:** On-chain smart contract bugs (upstream), social engineering, physical access

Keep it concise --- under 60 lines.

> **Research Insights (SECURITY.md scope):**
>
> The original scope listed 5 items. Security review identified 6 additional items specific to a crypto wallet tool:
>
> 1. **Safety ceiling bypass** --- `charge_raw()` is `pub` and skips all safety checks. Any downstream consumer can call it directly. This is the most significant trust boundary in the API.
> 2. **Configuration injection** --- `payg.toml` loads from CWD (`config.rs:43-52`). A malicious TOML in a cloned repo could redirect `recipient` or `facilitator_url`.
> 3. **Facilitator trust boundary** --- the x402 backend trusts the facilitator's response (documented in todo #007). A compromised facilitator can return a fake `tx_hash`.
> 4. **Keyfile confidentiality** --- TOCTOU race was fixed with `create_new(true)`, but file permission checks at load time are not enforced.
> 5. **Environment variable exfiltration** --- `PAYG_PRIVATE_KEY` is readable via `/proc/PID/environ` on shared systems.
> 6. **Network mismatch** --- a malicious `PAYG_NETWORK=base` env var could silently redirect to mainnet.
>
> These should all be listed as in-scope for security researchers.

#### 3b. Add crate metadata to workspace Cargo.toml

**File:** `Cargo.toml` (root)

Add to `[workspace.package]`:

```toml
repository = "https://github.com/brettdavies/payg"
homepage = "https://github.com/brettdavies/payg"
```

Both crates already inherit from `workspace.package` so they'll pick these up automatically. Also add corresponding `.workspace = true` entries in each crate's `Cargo.toml`:

```toml
repository.workspace = true
homepage.workspace = true
```

> **Research Insight:** `keywords` and `categories` are crates.io-only fields with zero value when `git_only = true`. Dropped per YAGNI --- add them when publishing to crates.io. The `repository` field is still useful: it appears in `cargo metadata` output and strengthens cargo-deny's source identification.

#### 3c. Add GitHub issue templates

**New directory:** `.github/ISSUE_TEMPLATE/`

Use YAML issue forms (`.yml`) instead of markdown templates (`.md`). YAML forms provide structured inputs, required field validation, and dropdowns.

**File:** `.github/ISSUE_TEMPLATE/bug_report.yml`

```yaml
name: Bug Report
description: Report a bug or unexpected behavior
labels: ["bug"]
body:
  - type: textarea
    id: description
    attributes:
      label: Description
      description: What happened?
    validations:
      required: true
  - type: textarea
    id: steps
    attributes:
      label: Steps to Reproduce
      description: Minimal steps to reproduce the issue
      placeholder: |
        1. Run `payg charge ...`
        2. ...
    validations:
      required: true
  - type: textarea
    id: expected
    attributes:
      label: Expected Behavior
    validations:
      required: true
  - type: textarea
    id: actual
    attributes:
      label: Actual Behavior
    validations:
      required: true
  - type: input
    id: version
    attributes:
      label: payg version
      placeholder: "e.g. 0.1.0"
    validations:
      required: true
  - type: dropdown
    id: network
    attributes:
      label: Network
      options:
        - base-sepolia
        - base
    validations:
      required: true
  - type: dropdown
    id: backend
    attributes:
      label: Backend
      options:
        - x402
        - eth
    validations:
      required: true
  - type: input
    id: os
    attributes:
      label: Operating System
      placeholder: "e.g. macOS 15, Ubuntu 24.04"
    validations:
      required: true
```

**File:** `.github/ISSUE_TEMPLATE/feature_request.yml`

```yaml
name: Feature Request
description: Suggest a new feature or improvement
labels: ["enhancement"]
body:
  - type: textarea
    id: problem
    attributes:
      label: Problem
      description: What problem does this solve?
    validations:
      required: true
  - type: textarea
    id: solution
    attributes:
      label: Proposed Solution
      description: How should it work?
    validations:
      required: true
  - type: textarea
    id: alternatives
    attributes:
      label: Alternatives Considered
      description: What other approaches were evaluated?
    validations:
      required: false
```

**File:** `.github/ISSUE_TEMPLATE/config.yml`

```yaml
blank_issues_enabled: false
contact_links:
  - name: Questions
    url: https://github.com/brettdavies/payg/discussions
    about: Ask questions and get help
```

**Note:** Verify GitHub Discussions is enabled on the repo. If not, either enable it or change the link target to the Issues tab.

> **Research Insight:** GitHub YAML issue forms (`.yml`) are the modern recommended format over markdown templates (`.md`). They provide structured `input`, `textarea`, `dropdown`, and `checkboxes` types with per-field `required` validation. Note: PR templates do NOT support YAML forms --- the existing markdown PR template is correct.

---

### Phase 4: Unit Tests and Doc-Tests

Focus on pure functions only. No HTTP mocking infrastructure --- the existing e2e/integration tests cover network interactions.

#### 4a. `check_safety_ceiling()` unit tests (HIGH PRIORITY)

**File:** `crates/payg/src/lib.rs` --- add `#[cfg(test)] mod tests`

This is the highest priority test because it has a known P1 bug (todo 025: cross-token ceiling comparison). The bug appears to be fixed (line 102 checks token mismatch), but tests must verify the fix and prevent regression.

Test cases:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::ConsumerConfig;
    use crate::pricing::parse_price;

    #[test]
    fn ceiling_allows_amount_under_limit() {
        // Default ceiling: 1.00 USDC. Charge: 0.001 USDC. Should pass.
        let parsed = parse_price("0.001 USDC").unwrap();
        let config = ConsumerConfig::default();
        assert!(check_safety_ceiling(&parsed, "0.001 USDC", &config).is_ok());
    }

    #[test]
    fn ceiling_allows_amount_at_limit() {
        // Default ceiling: 1.00 USDC. Charge: 1.00 USDC. Should pass (<=).
        let parsed = parse_price("1.00 USDC").unwrap();
        let config = ConsumerConfig::default();
        assert!(check_safety_ceiling(&parsed, "1.00 USDC", &config).is_ok());
    }

    #[test]
    fn ceiling_rejects_amount_over_limit() {
        // Default ceiling: 1.00 USDC. Charge: 2.00 USDC. Should fail.
        let parsed = parse_price("2.00 USDC").unwrap();
        let config = ConsumerConfig::default();
        let err = check_safety_ceiling(&parsed, "2.00 USDC", &config).unwrap_err();
        assert!(matches!(err, PaygError::ExceedsSafetyCeiling { .. }));
    }

    #[test]
    fn ceiling_rejects_cross_token_comparison() {
        // Default ceiling is USDC. Charging in ETH should error, NOT silently compare.
        let parsed = parse_price("0.001 ETH").unwrap();
        let config = ConsumerConfig::default();
        let err = check_safety_ceiling(&parsed, "0.001 ETH", &config).unwrap_err();
        assert!(matches!(err, PaygError::ConfigError(_)));
    }

    #[test]
    fn ceiling_allows_free() {
        let parsed = parse_price("free").unwrap();
        let config = ConsumerConfig::default();
        assert!(check_safety_ceiling(&parsed, "free", &config).is_ok());
    }
}
```

**Note:** `ConsumerConfig::default()` must produce a config with the default USDC ceiling. Verify this at implementation time. If `Default` isn't derived, construct manually with `max_charge: None` and the relevant defaults.

> **Research Insight (additional test case):** Security review recommends adding a test for `PAYG_MAX_CHARGE` environment variable override --- verify that a raised ceiling via env var is reflected in `check_safety_ceiling`. This exercises `config.rs:129-133` where the ceiling reads from the environment. Add during implementation if `ConsumerConfig` construction allows it without side effects.

#### 4b. `format_token_amount()` unit tests

**File:** `crates/payg-cli/src/commands/balance.rs` --- add `#[cfg(test)] mod tests`

The function is private (`fn format_token_amount`), so tests go in the same file. The function is parameterized by `decimals` (not branched), so USDC tests (6 decimals) sufficiently cover the logic. ETH-specific tests are redundant.

Test cases:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use alloy_primitives::U256;

    #[test]
    fn format_zero() {
        assert_eq!(format_token_amount(U256::ZERO, 6), "0.0");
    }

    #[test]
    fn format_one_usdc() {
        // 1.0 USDC = 1_000_000 (6 decimals)
        assert_eq!(format_token_amount(U256::from(1_000_000u64), 6), "1.0");
    }

    #[test]
    fn format_fractional_usdc() {
        // 0.001 USDC = 1000 (6 decimals)
        assert_eq!(format_token_amount(U256::from(1000u64), 6), "0.001");
    }

    #[test]
    fn format_large_amount() {
        // 1234.567890 USDC = 1_234_567_890 (6 decimals)
        assert_eq!(format_token_amount(U256::from(1_234_567_890u64), 6), "1234.56789");
    }
}
```

> **Research Insight:** Trimmed from 6 to 4 tests. The `format_one_eth` and `format_fractional_eth` tests were redundant --- the function is parameterized by `decimals`, so if 6-decimal division works, 18-decimal division works identically. No branching logic depends on the decimal count.

#### 4c. Doc-tests on public API

**File:** `crates/payg/src/lib.rs`

Add doc-test to `check_safety_ceiling` (pure, synchronous, easy to demonstrate):

```rust
/// Check that a parsed price does not exceed the safety ceiling.
///
/// The ceiling and charge must use the same token. If the ceiling is in USDC
/// and the charge is in ETH (or vice versa), the check returns an error.
///
/// # Examples
///
/// ```
/// use payg::config::ConsumerConfig;
/// use payg::pricing::parse_price;
/// use payg::check_safety_ceiling;
///
/// let price = parse_price("0.50 USDC").unwrap();
/// let config = ConsumerConfig::default();
/// assert!(check_safety_ceiling(&price, "0.50 USDC", &config).is_ok());
/// ```
```

**File:** `crates/payg/src/pricing.rs`

Add doc-test to `parse_price`:

```rust
/// # Examples
///
/// ```
/// use payg::pricing::parse_price;
///
/// let price = parse_price("0.001 USDC").unwrap();
/// assert_eq!(price.token, "USDC");
///
/// let free = parse_price("free").unwrap();
/// assert!(free.amount.is_zero());
/// ```
```

The async `charge*` functions are not good doc-test candidates --- they require a wallet, network, and live facilitator. Skip them.

> **Research Insight:** The `eip712_extra` test from the original plan has been dropped. The function (`x402.rs:139-144`) is a 4-line field copy with no branching logic --- the test would just restate the implementation. The on-chain integration tests already exercise this path end-to-end.

---

### Phase 5: Todo Audit

#### 5a. Audit pending todos against code

For each pending todo, check whether the issue has been fixed in the current codebase. The research suggests many P1 items are already resolved:

**Likely already fixed (verify during implementation):**

| Todo | Issue | Evidence |
|------|-------|----------|
| 001 | TOCTOU race in keyfile creation | `init.rs` uses `create_new(true)` (atomic create-or-fail) |
| 002 | URL validation SSRF | `config.rs` has `validate_url()` with HTTPS enforcement |
| 003 | HTTP timeouts | x402 backend has 30s/10s timeouts; balance queries have same |
| 004 | Init JSON output | Needs verification |
| 005 | JSON error stdout | Needs verification |
| 006 | Zeroize private keys | `wallet.rs` uses `Zeroizing<String>` |
| 025 | Cross-token ceiling | `lib.rs:102` checks token mismatch |

**Process:**

1. For each pending todo, read the referenced source file
2. If the issue is fixed, rename the file from `NNN-pending-*` to `NNN-complete-*`
3. If the issue is NOT fixed, leave as pending (these become post-release work items)
4. Document the audit results in a brief summary

**Scope guard:** Only audit and reclassify. Do NOT fix any open issues discovered during the audit --- that's separate work.

> **Research Insight (todo convention):** The `todos/` directory is gitignored and uses the naming pattern `NNN-status-priority-description.md`. Next available ID is 043. When transitioning `pending` to `complete`, rename the file (e.g., `025-pending-p1-*` becomes `025-complete-p1-*`). Use the newer YAML frontmatter format (as seen in todo 026+) for any new entries.

---

## Acceptance Criteria

### Phase 1

- [ ] README license section matches `Cargo.toml` license field
- [ ] Anchor tag `v0.1.0` exists on `main` (done LAST, after all changes merge)

### Phase 2

- [ ] All `actions/checkout` and `Swatinem/rust-cache` uses in `ci.yml` are SHA-pinned
- [ ] `deny.toml` exists with `ring` clarification and `cargo deny check` passes locally
- [ ] `deny` job exists in `ci.yml` with `rust-version: "1.88.0"` and is added to branch ruleset required checks
- [ ] Dependabot monitors both `github-actions` and `cargo` ecosystems

### Phase 3

- [ ] `SECURITY.md` exists with expanded disclosure policy (10 scope items)
- [ ] `Cargo.toml` has `repository` and `homepage` in `[workspace.package]`
- [ ] `.github/ISSUE_TEMPLATE/` contains `bug_report.yml`, `feature_request.yml`, and `config.yml`

### Phase 4

- [ ] `check_safety_ceiling` has unit tests covering: under limit, at limit, over limit, cross-token rejection, free
- [ ] `format_token_amount` has 4 unit tests covering: zero, whole numbers, fractional, large amounts
- [ ] `check_safety_ceiling` and `parse_price` have doc-tests
- [ ] `cargo test` and `cargo test --all-features` pass

### Phase 5

- [ ] All 31 pending todos audited against current code
- [ ] Fixed items renamed from `pending` to `complete`
- [ ] Remaining open items left as-is for post-release work

## Dependencies and Risks

- **`ring` license hash:** The `0xbd0eed23` hash in `deny.toml` must match the actual `ring` v0.17.14 LICENSE file. If the hash is wrong, `cargo deny check licenses` will fail with a clear error message. Verify at implementation time by running `cargo deny check licenses` locally.
- **cargo-deny removed fields:** Do NOT use `vulnerability`, `notice`, or `severity-threshold` in `[advisories]` --- they cause hard errors in cargo-deny 0.16+.
- **CC0-1.0 exceptions:** The `tiny-keccak`, `secp256k1`, and `secp256k1-sys` exceptions must match the actual crate names in `Cargo.lock`. Verify with `cargo deny check licenses` locally.
- **GitHub Discussions:** Issue template `config.yml` links to Discussions. If not enabled on the repo, either enable it or change the link target to the Issues tab.
- **Anchor tag timing:** Must be the LAST step. Creating it before all changes merge means the first changelog won't include this hardening work.
- **`ConsumerConfig::default()`:** Unit tests for `check_safety_ceiling` assume `Default` is derived or manually implemented. Verify at implementation time.
- **Ruleset staging:** Add `deny` to `protect-main.json` required checks AFTER verifying the job passes on at least one PR. Prevents chicken-and-egg where the ruleset requires a check that doesn't yet exist.

## References

### Internal

- Brainstorm: `docs/brainstorms/2026-02-18-pre-release-hardening-brainstorm.md`
- Release-plz setup: `docs/solutions/integration-issues/release-plz-rust-workspace-setup.md`
- CI feature matrix: `docs/solutions/build-errors/rust-ci-feature-matrix-additive-gotcha.md`
- Phase 3 review: `docs/solutions/code-review/phase3-multi-agent-review-findings.md`

### External

- cargo-deny docs: <https://embarkstudios.github.io/cargo-deny/>
- cargo-deny-action: <https://github.com/EmbarkStudios/cargo-deny-action>
- alloy-rs deny.toml (reference): <https://github.com/alloy-rs/alloy/blob/main/deny.toml>
- Foundry deny.toml (reference): <https://github.com/foundry-rs/foundry/blob/master/deny.toml>
- GitHub Security Advisories: <https://docs.github.com/en/code-security/security-advisories>
- GitHub YAML issue forms: <https://docs.github.com/en/communities/using-templates-to-encourage-useful-issues-and-pull-requests/syntax-for-issue-forms>
- CVE-2025-30066 (action tag compromise): <https://www.cisa.gov/news-events/alerts/2025/03/18/supply-chain-compromise-third-party-github-action-cve-2025-30066>
