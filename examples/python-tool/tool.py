#!/usr/bin/env python3
"""Example: Python tool gating execution behind PAYG payment.

Shells out to `payg charge --output json`, parses the response, and
only runs the tool logic if payment succeeds.

Requires: payg CLI installed and wallet configured (PAYG_PRIVATE_KEY or keyfile).
"""

import json
import subprocess
import sys


def charge(amount: str, dry_run: bool = False) -> dict:
    """Run `payg charge` and return the parsed JSON response."""
    cmd = ["payg", "charge", amount, "--output", "json"]
    if dry_run:
        cmd.append("--dry-run")
    result = subprocess.run(cmd, capture_output=True, text=True)

    try:
        response = json.loads(result.stdout)
    except json.JSONDecodeError:
        print(f"payg error: {result.stderr.strip()}", file=sys.stderr)
        sys.exit(result.returncode or 1)

    if response.get("status") not in ("success", "dry_run"):
        code = response.get("code", "UNKNOWN")
        message = response.get("message", "payment failed")
        print(f"Payment failed [{code}]: {message}", file=sys.stderr)
        sys.exit(result.returncode or 42)

    return response


def main():
    if len(sys.argv) < 2:
        print("Usage: tool.py <text to reverse>", file=sys.stderr)
        sys.exit(1)

    text = " ".join(sys.argv[1:])

    # Dry-run first to estimate cost (recommended for agents)
    estimate = charge("0.001 USDC", dry_run=True)
    print(f"Estimated cost: {estimate.get('amount', 'unknown')}", file=sys.stderr)

    # Charge for real
    receipt = charge("0.001 USDC")
    tx_hash = receipt["tx_hash"]

    # Tool logic: reverse the input text
    reversed_text = text[::-1]
    print(f"Reversed: {reversed_text}")
    print(f"Payment tx: {tx_hash}")


if __name__ == "__main__":
    main()
