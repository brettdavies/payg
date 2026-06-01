# Pre-Release Hardening

**Date:** 2026-02-18
**Status:** Reviewed

## What We're Building

A set of improvements to harden the repo before the initial v0.1.0 release (git-only, not crates.io). The repo is functionally complete --- this work focuses on security posture, CI reliability, documentation accuracy, test coverage, and contributor experience.

## Why This Approach

The repo handles private keys and financial transactions. Security gaps (no dependency auditing, no vulnerability disclosure policy) and inconsistent CI hardening (mutable action tags) present real risk. The todo backlog is out of sync with the code, creating false alarm noise. And the test suite has gaps in security-critical modules that only have e2e coverage.

## Key Decisions

### 1. Fix README license mismatch (given)

The README license section says "MIT" but the repo was changed to dual MIT/Apache-2.0 in commit `638c17d`. Fix the README to match.

### 2. Create anchor git tag v0.1.0 (given)

Without an anchor tag, release-plz will dump all historical commits into the first changelog. The project's own solution doc (`docs/solutions/integration-issues/release-plz-rust-workspace-setup.md`) calls this out explicitly. The tag should be created on `main` at the current state, which represents the release-ready baseline.

### 3. Add SECURITY.md

Vulnerability disclosure policy. Standard for any project handling secrets and financial operations. Include:

- Supported versions
- How to report vulnerabilities (private disclosure via GitHub Security Advisories or email)
- Expected response timeline
- Scope of what constitutes a security issue

### 4. Add cargo-audit or cargo-deny to CI

Supply chain security scanning for the 485-dependency tree. Options:

- **cargo-audit**: Focused on known vulnerabilities (RustSec advisory DB)
- **cargo-deny**: Broader --- covers vulnerabilities, license compliance, banned crates, duplicate detection

**Decision:** Evaluate both during planning. cargo-deny is more comprehensive but cargo-audit is simpler. Either works.

### 5. SHA-pin CI workflow actions

The CI workflow (`ci.yml`) uses mutable tags (`@v4`, `@v2`) while the release workflow correctly SHA-pins everything. Align them for consistent supply chain security.

### 6. Add Cargo ecosystem to Dependabot

Currently only GitHub Actions are monitored. Add a `cargo` ecosystem entry to `dependabot.yml` for weekly Rust dependency update PRs.

### 7. Add crate metadata to Cargo.toml

Add `repository`, `homepage`, `keywords`, and `categories` to the workspace `Cargo.toml`. Even though `git_only = true`, this metadata appears in `cargo metadata` output and prepares for eventual crates.io publication.

### 8. Audit and reconcile todos

The `todos/` directory has 31 pending items, but many P1 items (TOCTOU race, URL validation, HTTP timeouts, zeroize) appear to already be fixed in the code. Reconcile the backlog with actual code state --- close fixed items, verify open items are still valid.

### 9. Add GitHub issue templates

Three files in `.github/ISSUE_TEMPLATE/`:

- `bug_report.md` --- steps to reproduce, expected/actual behavior, environment info
- `feature_request.md` --- problem description, proposed solution, alternatives considered
- `config.yml` --- disable blank issues, add external links (confirm GitHub Discussions is enabled, or link to Issues as fallback)

### 10. Add unit tests for untested modules + doc-tests

Current gaps with zero unit tests:

- `wallet.rs` --- key loading, keyfile decryption, environment variable handling
- `backend/x402.rs` --- x402 facilitator request building, response parsing
- `backend/eth.rs` --- ETH transfer construction
- `backend.rs` --- feature-gated dispatch logic
- `charge.rs` --- orchestration flow

Also add doc-tests (`///` examples) on the public API surface in `lib.rs` so examples are verified by `cargo test`.

**Scope note:** The backend modules (`x402.rs`, `eth.rs`) make HTTP and blockchain calls. Unit testing them in isolation requires a mocking strategy (e.g., `mockito`, trait-based injection, or test doubles). The specific approach should be decided during planning. Focus unit tests on pure logic paths (request construction, response parsing, error mapping) rather than full round-trip mocking. The existing e2e/integration tests already cover the happy-path network interactions.

## What We're NOT Doing

- **CONTRIBUTING.md** --- Deferred to post-release
- **CODEOWNERS** --- Solo project, not needed yet
- **Code coverage tracking** (tarpaulin/llvm-cov) --- Deferred
- **cargo doc CI check** --- Already tracked in todos, deferred
- **Publishing to crates.io** --- Explicitly out of scope

## Suggested Implementation Order

Items are grouped by logical dependency and risk:

1. **Fixes first:** README license, anchor tag (unblocks release-plz)
2. **CI hardening:** SHA-pin actions, add cargo-audit/cargo-deny, add Cargo Dependabot
3. **Documentation:** SECURITY.md, crate metadata, issue templates
4. **Testing:** Unit tests for untested modules, doc-tests
5. **Cleanup:** Todo audit and reconciliation

## Open Questions

None --- all decisions resolved during brainstorming.
