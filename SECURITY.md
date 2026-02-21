# Security Policy

## Supported Versions

| Version | Supported |
|---------|-----------|
| 0.1.x   | Yes       |

## Reporting a Vulnerability

**Preferred:** Use GitHub's [Security Advisories](https://github.com/brettdavies/payg/security/advisories/new) to report vulnerabilities privately.

**Alternative:** Email the maintainer with the subject line `[SECURITY] payg` and include:

- Description of the vulnerability
- Steps to reproduce
- Affected version(s)

### Response Timeline

- **Acknowledge:** within 48 hours
- **Patch critical issues:** within 7 days
- **Patch non-critical issues:** next release cycle

## Scope

The following areas are in scope for security reports:

1. Private key handling and keyfile confidentiality
2. Wallet operations and key loading
3. Payment execution and receipt integrity
4. Safety ceiling bypass or circumvention
5. Configuration injection via `payg.toml` or environment variables
6. Facilitator response integrity (fake `tx_hash`)
7. Private key exposure via environment variables
8. Network mismatch leading to unintended mainnet transactions
9. URL validation bypass
10. Dependency vulnerabilities

## Out of Scope

- On-chain smart contract bugs (upstream)
- Social engineering
- Physical access to the machine
