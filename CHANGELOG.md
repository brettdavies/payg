# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.0](https://github.com/brettdavies/payg/releases/tag/v0.1.0) - 2026-02-21

### Bug Fixes

- resolve 8 review findings across security, correctness, and CI

### Documentation

- add SECURITY.md, crate metadata, and issue templates

### Features

- *(network)* add configurable network selection for Base mainnet and testnet
- *(v1)* complete Phase 3 — polish, CI, tests, examples ([#5](https://github.com/brettdavies/payg/pull/5))
- *(release)* add release-plz automation with workspace version inheritance ([#6](https://github.com/brettdavies/payg/pull/6))

### Testing

- *(on-chain)* harden e2e tests with security guards, LazyLock client, and on-chain receipt verification ([#4](https://github.com/brettdavies/payg/pull/4))
- add unit tests and doc-tests for safety ceiling and formatting
