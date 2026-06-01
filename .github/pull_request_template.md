## Summary

<!-- Brief overview: what does this PR introduce?

     SCOPE: Describe the net diff only — what the merged result looks
     like compared to the base branch. NOT commit history, intermediate
     state, or how the cherry-picks were assembled.

     EXCLUDE all verification artifacts:
- Triple-diff output / stats (A, B, C blocks)
- Leak-check output ("no guarded paths leaked", "guard-main-docs runs clean")
- Patch-id cherry-check counts
- Pre-push gate results, CI status, prose-scrub findings
- Any "I ran X and it returned Y" narration

     Anomalies get fixed before push, not audit-trailed in the body.
-->

## Type of Change

- [ ] `feat`: New feature
- [ ] `fix`: Bug fix
- [ ] `refactor`: Code refactoring (no functional changes)
- [ ] `style`: Formatting changes (no functional changes)
- [ ] `perf`: Performance improvement
- [ ] `docs`: Documentation update
- [ ] `test`: Adding or updating tests
- [ ] `chore`: Maintenance tasks (dependencies, config, etc.)
- [ ] `ci`: CI/CD configuration changes
- [ ] `build`: Build system changes

## Testing

- [ ] `cargo test --all-features` passing
- [ ] `cargo clippy --all-targets --all-features -- -D warnings` clean
- [ ] `cargo fmt --all --check` clean

## Checklist

- [ ] Code follows project conventions (see `AGENTS.md`)
- [ ] Commit messages follow [Conventional Commits](https://www.conventionalcommits.org/)
- [ ] Tests added/updated and passing

<!--
PR Title Format: <type>(<scope>): <description>

Examples:
- feat(wallet): add multi-key support
- fix(cli): resolve config file precedence
- docs(readme): update installation instructions

Breaking changes: add a BREAKING CHANGE footer to the commit message body. See
https://www.conventionalcommits.org/en/v1.0.0/#specification -->
