---
title: "Second review pass: clap try_parse for JSON errors and agent-native parity fixes"
type: code-review-process
date: 2026-02-11
component: payg-cli (clap integration, JSON error handling, agent-native output)
tags:
  - code-review
  - agent-native
  - json-output
  - clap
  - error-handling
  - environment-variables
severity: P1 + P2
symptoms: >
  7-agent second-pass code review of PR #3 (feat/network-config) identified 1 P1
  and 5 P2 agent-native parity gaps: clap parse() bypasses JSON error handler,
  is_testnet missing from JSON output, 5 of 7 env vars undiscoverable via --help,
  error JSON missing network field, help text gaps.
root_cause: >
  Cli::parse() calls process::exit() directly on argument errors, so the custom
  JSON error handler (which checks --output/PAYG_OUTPUT) never runs. Additionally,
  JSON output schemas were incomplete (missing is_testnet, network in errors) and
  env var documentation was only in source comments, not --help.
resolution: >
  All 6 findings fixed in one commit: try_parse() with PAYG_OUTPUT env var check,
  is_testnet added to all 6 JSON output paths, network included in error JSON,
  ENVIRONMENT VARIABLES section added to --help, help text updated, doc comment added.
---

# Second Review Pass: Agent-Native Parity Fixes

## Problem

A 7-agent parallel code review (security, performance, architecture, patterns, simplicity, agent-native, git-history) on PR #3 after the first pass of 17 fixes found additional issues. After deduplication across agents, 1 P1 and 5 P2 findings remained:

| Priority | Finding | Impact |
|----------|---------|--------|
| P1 | `Cli::parse()` bypasses JSON error handler | Agents get plain text on stderr for argument errors |
| P2 | `--non-interactive` help missing `PAYG_KEY_PASSWORD` | Agents must fail once to discover required env var |
| P2 | No `is_testnet` in JSON output | Agents can't programmatically detect mainnet |
| P2 | 5 of 7 env vars not in `--help` | Agents can't discover configuration options |
| P2 | Error JSON missing `network` field | Agents can't determine failure context |
| P2 | `resolve_network()` dual-read undocumented | Developer confusion about precedence chain |

6 of 7 review agents said "merge". The agent-native reviewer was the sole dissenter.

## Solution

### 1. try_parse() for JSON Error Handling (P1)

```rust
// Before: parse() exits directly, bypasses JSON handler
let cli = Cli::parse();

// After: try_parse() catches errors, respects PAYG_OUTPUT
let cli = match Cli::try_parse() {
    Ok(cli) => cli,
    Err(e) => {
        // Help/version: print normally regardless of output format
        if matches!(
            e.kind(),
            clap::error::ErrorKind::DisplayHelp | clap::error::ErrorKind::DisplayVersion
        ) {
            e.exit();
        }
        // Argument errors: respect PAYG_OUTPUT env var for JSON
        let is_json = std::env::var("PAYG_OUTPUT")
            .map(|v| v.eq_ignore_ascii_case("json"))
            .unwrap_or(false);
        if is_json {
            let err_json = serde_json::json!({
                "status": "error",
                "code": "INVALID_ARGS",
                "message": e.to_string(),
            });
            println!("{}", serde_json::to_string(&err_json).unwrap());
            std::process::exit(2);
        }
        e.exit();
    }
};
```

Key decisions:
- Check `PAYG_OUTPUT` env var (not `--output` flag) since clap couldn't parse the args
- Case-insensitive check (`eq_ignore_ascii_case`) for robustness
- `--help` and `--version` print normally regardless of output format
- Exit code 2 preserved for argument errors (Unix convention)

### 2. is_testnet in All JSON Outputs (P2)

Added `"is_testnet": network.is_testnet` to all 6 JSON output paths:

```rust
// charge (dry_run + success), balance, address, init
let result = serde_json::json!({
    "status": "ok",
    "network": network.name,
    "is_testnet": network.is_testnet,  // NEW
    // ...other fields
});
```

Agents can now detect mainnet programmatically:
```bash
IS_TESTNET=$(payg address --output json | jq -r '.is_testnet')
[[ "$IS_TESTNET" == "true" ]] || echo "WARNING: mainnet"
```

### 3. Network in Error JSON (P2)

```rust
if let Err(e) = result {
    let mut err = serde_json::json!({
        "status": "error",
        "code": error_code(&e),
        "message": e.to_string(),
    });
    // Include network context when available
    if let Some(name) = network_str {
        err["network"] = serde_json::json!(name);
    }
    println!("{}", serde_json::to_string(&err).unwrap());
}
```

Network is included when available (from `--network` flag or `PAYG_NETWORK` env var). If network resolution itself failed, the field is omitted.

### 4. Environment Variables in --help (P2)

Added `ENVIRONMENT VARIABLES` section to clap's `after_help`:

```
ENVIRONMENT VARIABLES:
  PAYG_OUTPUT            Output format: text, json (same as --output)
  PAYG_NETWORK           Target network: base, base-sepolia (same as --network)
  PAYG_PRIVATE_KEY       Hex private key (recommended for agents, bypasses keyfile)
  PAYG_KEY_PASSWORD      Keyfile decryption password (avoids interactive prompt)
  PAYG_MAX_CHARGE        Safety ceiling override (e.g. "5.00 USDC")
  PAYG_RPC_URL           RPC endpoint override
  PAYG_FACILITATOR_URL   x402 facilitator URL override
```

### 5. Help Text & Doc Comment (P2)

- `--non-interactive` changed from "uses env vars" to "requires PAYG_KEY_PASSWORD env var"
- `resolve_network()` doc comment now explains the full precedence chain:
  `--network` flag > `PAYG_NETWORK` env var > config `network` field > default

## Prevention & Best Practices

### 1. Always Use try_parse() When Custom Error Handling Exists

**Trigger**: Any Rust CLI using clap with `--output json`, structured logging, or agent integration.

`Cli::parse()` calls `process::exit()` directly. If your CLI has any custom error formatting, you MUST use `try_parse()` instead. This applies to any output mode beyond plain stderr text.

### 2. Semantic JSON Fields Over stderr Warnings

**Trigger**: Any CLI warning that agents need to see.

Never use `eprintln!()` for information agents need. Add semantic fields to JSON output instead. Reserve stderr for human-only context (progress bars, debug output).

| Information Type | Human (text) | Agent (JSON) |
|-----------------|--------------|--------------|
| Mainnet warning | `eprintln!("WARNING: mainnet")` | `"is_testnet": false` |
| Network name | `println!("Network: Base")` | `"network": "base"` |
| Error context | `eprintln!("error: ...")` | `{"status": "error", "code": "...", "network": "..."}` |

### 3. Document All Env Vars in --help

**Trigger**: Any CLI accepting environment variables.

If an env var exists, it must appear in `--help`. Use clap's `after_help` for an `ENVIRONMENT VARIABLES` section. Include: name, purpose, format, and relationship to CLI flags.

### 4. Consistent JSON Schema Across All Paths

**Trigger**: Multiple JSON output paths (success/error, different commands).

Error responses should include the same context fields as success responses when possible. Agents shouldn't need different parsing logic for errors vs success.

### 5. Document Dual-Read Patterns

**Trigger**: Same env var read at multiple code layers (CLI + lib).

When an env var is consumed at two layers, add explicit doc comments explaining:
- Why dual-read exists
- Which layer takes precedence
- The full precedence chain

## Results

| Metric | Value |
|--------|-------|
| Findings addressed | 6/6 (1 P1 + 5 P2) |
| Fix commits | 1 |
| Tests after fixes | 14 unit + 1 sync test passing, 13 integration ignored |
| LOC added | 44 lines across 7 files |
| Clippy/fmt | Clean |

## Related Documentation

- [Network Config Review (Pass 1)](../code-review/network-config-review-and-systematic-fix-batching.md) - First review: 17 findings fixed across 4 batches
- [V1 Multi-Agent Review](../code-review/v1-multi-agent-review-resolution.md) - Initial v1 review: 25 findings
- [Network Config Plan](../../plans/2026-02-10-feat-configurable-network-for-testnet-support-plan.md) - Original implementation plan
- [Rust Workspace Build Errors](../build-errors/rust-workspace-dependency-conflicts-and-feature-flags.md) - Build error patterns in same workspace
