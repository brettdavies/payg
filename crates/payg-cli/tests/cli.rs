// assert_cmd::Command::cargo_bin is deprecated but no stable replacement exists yet
#[allow(deprecated)]
use assert_cmd::Command;
use predicates::prelude::*;

/// Well-known Hardhat account #0 private key (public test key, no real funds).
const HARDHAT_KEY: &str = "0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80";
const HARDHAT_ADDR: &str = "0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266";

/// Standard payg.toml fixture for charge tests.
const PAYG_TOML_FIXTURE: &str = "\
recipient = \"0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266\"\n\
default_price = \"0.001 USDC\"\n";

/// Build a `payg` command isolated from user config.
///
/// Uses a fresh temp dir as HOME so stale keyfiles or config from previous
/// test runs cannot contaminate results. Callers must keep the returned
/// TempDir alive for the duration of the test (drop = cleanup).
fn base_cmd() -> (Command, tempfile::TempDir) {
    let fake_home = tempfile::tempdir().unwrap();
    #[allow(deprecated)]
    let mut cmd = Command::cargo_bin("payg").unwrap();
    cmd.env("HOME", fake_home.path());
    cmd.env_remove("PAYG_PRIVATE_KEY");
    cmd.env_remove("PAYG_NETWORK");
    cmd.env_remove("PAYG_OUTPUT");
    cmd.env_remove("PAYG_MAX_CHARGE");
    cmd.env_remove("PAYG_RPC_URL");
    cmd.env_remove("PAYG_FACILITATOR_URL");
    cmd.env_remove("PAYG_KEY_PASSWORD");
    cmd.env_remove("RUST_LOG");
    (cmd, fake_home)
}

/// Create a temp dir with a payg.toml containing the given content.
fn temp_project(toml_content: &str) -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("payg.toml"), toml_content).unwrap();
    dir
}

// ── Version & help ──────────────────────────────────────────────────

#[test]
fn version_flag() {
    let (mut cmd, _home) = base_cmd();
    cmd.arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::contains("payg"));
}

#[test]
fn help_output() {
    let (mut cmd, _home) = base_cmd();
    cmd.arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("init"))
        .stdout(predicate::str::contains("charge"))
        .stdout(predicate::str::contains("address"))
        .stdout(predicate::str::contains("balance"))
        .stdout(predicate::str::contains("PAYG_PRIVATE_KEY"));
}

#[test]
fn invalid_subcommand_exits_2() {
    let (mut cmd, _home) = base_cmd();
    cmd.arg("nonexistent").assert().code(2);
}

// ── Address command ─────────────────────────────────────────────────

#[test]
fn address_deterministic() {
    let (mut cmd, _home) = base_cmd();
    cmd.env("PAYG_PRIVATE_KEY", HARDHAT_KEY)
        .arg("address")
        .assert()
        .success()
        .stdout(predicate::str::contains(HARDHAT_ADDR));
}

#[test]
fn address_json_format() {
    let (mut cmd, _home) = base_cmd();
    let output = cmd
        .env("PAYG_PRIVATE_KEY", HARDHAT_KEY)
        .args(["address", "--output", "json"])
        .output()
        .unwrap();

    assert!(output.status.success());
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(json["status"], "ok");
    assert_eq!(json["address"], HARDHAT_ADDR);
    let obj = json.as_object().unwrap();
    assert!(obj.contains_key("status"), "missing 'status' field");
    assert!(obj.contains_key("address"), "missing 'address' field");
}

#[test]
fn address_json_via_env_var() {
    let (mut cmd, _home) = base_cmd();
    let output = cmd
        .env("PAYG_PRIVATE_KEY", HARDHAT_KEY)
        .env("PAYG_OUTPUT", "json")
        .arg("address")
        .output()
        .unwrap();

    assert!(output.status.success());
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(json["status"], "ok");
    assert_eq!(json["address"], HARDHAT_ADDR);
}

#[test]
fn address_no_wallet_exits_77() {
    let (mut cmd, _home) = base_cmd();
    cmd.arg("address").assert().code(77);
}

#[test]
fn address_no_wallet_json_error() {
    let (mut cmd, _home) = base_cmd();
    let output = cmd.args(["address", "--output", "json"]).output().unwrap();

    assert_eq!(output.status.code(), Some(77));
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(json["status"], "error");
    assert_eq!(json["code"], "NO_WALLET");
    assert!(json["message"].is_string(), "missing 'message' field");
}

// ── Balance command ─────────────────────────────────────────────────

#[test]
fn balance_no_wallet_exits_77() {
    let (mut cmd, _home) = base_cmd();
    cmd.arg("balance").assert().code(77);
}

#[test]
fn balance_no_wallet_json_error() {
    let (mut cmd, _home) = base_cmd();
    let output = cmd.args(["balance", "--output", "json"]).output().unwrap();

    assert_eq!(output.status.code(), Some(77));
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(json["status"], "error");
    assert_eq!(json["code"], "NO_WALLET");
}

// ── Charge command ──────────────────────────────────────────────────

#[test]
fn charge_dry_run_text() {
    let dir = temp_project(PAYG_TOML_FIXTURE);

    let (mut cmd, _home) = base_cmd();
    cmd.env("PAYG_PRIVATE_KEY", HARDHAT_KEY)
        .current_dir(dir.path())
        .args(["charge", "--dry-run"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Dry run"));
}

#[test]
fn charge_dry_run_json() {
    let dir = temp_project(PAYG_TOML_FIXTURE);

    let (mut cmd, _home) = base_cmd();
    let output = cmd
        .env("PAYG_PRIVATE_KEY", HARDHAT_KEY)
        .current_dir(dir.path())
        .args(["charge", "--dry-run", "--output", "json"])
        .output()
        .unwrap();

    assert!(output.status.success());
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(json["status"], "dry_run");
    assert!(json["amount"].is_string(), "missing 'amount' field");
    assert!(json["recipient"].is_string(), "missing 'recipient' field");
}

#[test]
fn charge_no_config_exits_78() {
    let dir = tempfile::tempdir().unwrap();

    let (mut cmd, _home) = base_cmd();
    // No payg.toml, no CLI amount/recipient — should fail with config error
    cmd.env("PAYG_PRIVATE_KEY", HARDHAT_KEY)
        .current_dir(dir.path())
        .arg("charge")
        .assert()
        .code(78);
}

#[test]
fn charge_exceeds_ceiling_exits_42() {
    let dir = temp_project("recipient = \"0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266\"\n");

    let (mut cmd, _home) = base_cmd();
    // Default ceiling is 1.00 USDC; charging 100 USDC exceeds it
    cmd.env("PAYG_PRIVATE_KEY", HARDHAT_KEY)
        .current_dir(dir.path())
        .args(["charge", "100 USDC"])
        .assert()
        .code(42);
}

// ── Init command ────────────────────────────────────────────────────

#[test]
fn init_already_exists_error() {
    let dir = temp_project(PAYG_TOML_FIXTURE);

    let (mut cmd, _home) = base_cmd();
    // payg.toml already present — init should fail
    cmd.env("PAYG_PRIVATE_KEY", HARDHAT_KEY)
        .current_dir(dir.path())
        .arg("init")
        .assert()
        .failure();
}
