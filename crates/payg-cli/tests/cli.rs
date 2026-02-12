#[allow(deprecated)]
use assert_cmd::Command;
use predicates::prelude::*;

/// Well-known Hardhat account #0 private key (public test key, no real funds).
const HARDHAT_KEY: &str = "0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80";
const HARDHAT_ADDR: &str = "0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266";

/// Non-existent HOME to isolate from user's real ~/.payg/config.toml.
const FAKE_HOME: &str = "/tmp/payg-test-nonexistent";

/// Build a `payg` command isolated from user config.
fn base_cmd() -> Command {
    #[allow(deprecated)]
    let mut cmd = Command::cargo_bin("payg").unwrap();
    cmd.env("HOME", FAKE_HOME);
    cmd.env_remove("PAYG_PRIVATE_KEY");
    cmd.env_remove("PAYG_NETWORK");
    cmd.env_remove("PAYG_OUTPUT");
    cmd.env_remove("PAYG_MAX_CHARGE");
    cmd.env_remove("PAYG_RPC_URL");
    cmd.env_remove("PAYG_FACILITATOR_URL");
    cmd.env_remove("PAYG_KEY_PASSWORD");
    cmd
}

// ── Version & help ──────────────────────────────────────────────────

#[test]
fn version_flag() {
    base_cmd()
        .arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::contains("payg"));
}

#[test]
fn help_lists_subcommands() {
    base_cmd()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("init"))
        .stdout(predicate::str::contains("charge"))
        .stdout(predicate::str::contains("address"))
        .stdout(predicate::str::contains("balance"));
}

#[test]
fn help_lists_env_vars() {
    base_cmd()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("PAYG_PRIVATE_KEY"));
}

#[test]
fn invalid_subcommand_exits_2() {
    base_cmd().arg("nonexistent").assert().code(2);
}

// ── Address command ─────────────────────────────────────────────────

#[test]
fn address_deterministic() {
    base_cmd()
        .env("PAYG_PRIVATE_KEY", HARDHAT_KEY)
        .arg("address")
        .assert()
        .success()
        .stdout(predicate::str::contains(HARDHAT_ADDR));
}

#[test]
fn address_json_format() {
    let output = base_cmd()
        .env("PAYG_PRIVATE_KEY", HARDHAT_KEY)
        .args(["address", "--output", "json"])
        .output()
        .unwrap();

    assert!(output.status.success());
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(json["status"], "ok");
    assert_eq!(json["address"], HARDHAT_ADDR);
}

#[test]
fn address_no_wallet_exits_77() {
    base_cmd().arg("address").assert().code(77);
}

#[test]
fn address_no_wallet_json_error() {
    let output = base_cmd()
        .args(["address", "--output", "json"])
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(77));
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(json["code"], "NO_WALLET");
}

// ── Charge command ──────────────────────────────────────────────────

#[test]
fn charge_dry_run_text() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("payg.toml"),
        "recipient = \"0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266\"\ndefault_price = \"0.001 USDC\"\n",
    )
    .unwrap();

    base_cmd()
        .env("PAYG_PRIVATE_KEY", HARDHAT_KEY)
        .current_dir(dir.path())
        .args(["charge", "--dry-run"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Dry run"));
}

#[test]
fn charge_dry_run_json() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("payg.toml"),
        "recipient = \"0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266\"\ndefault_price = \"0.001 USDC\"\n",
    )
    .unwrap();

    let output = base_cmd()
        .env("PAYG_PRIVATE_KEY", HARDHAT_KEY)
        .current_dir(dir.path())
        .args(["charge", "--dry-run", "--output", "json"])
        .output()
        .unwrap();

    assert!(output.status.success());
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(json["status"], "dry_run");
}

#[test]
fn charge_no_config_exits_78() {
    let dir = tempfile::tempdir().unwrap();
    // No payg.toml, no CLI amount/recipient — should fail with config error
    base_cmd()
        .env("PAYG_PRIVATE_KEY", HARDHAT_KEY)
        .current_dir(dir.path())
        .arg("charge")
        .assert()
        .code(78);
}

#[test]
fn charge_exceeds_ceiling_exits_42() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("payg.toml"),
        "recipient = \"0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266\"\n",
    )
    .unwrap();

    // Default ceiling is 1.00 USDC; charging 100 USDC exceeds it
    base_cmd()
        .env("PAYG_PRIVATE_KEY", HARDHAT_KEY)
        .current_dir(dir.path())
        .args(["charge", "100 USDC"])
        .assert()
        .code(42);
}
