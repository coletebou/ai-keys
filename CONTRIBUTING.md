# Contributing to ai-keys

Thanks for your interest in contributing to ai-keys.

## Getting Started

1. Fork the repository
2. Clone your fork: `git clone https://github.com/YOUR_USERNAME/ai-keys.git`
3. Create a branch: `git switch -c my-feature`
4. Make your changes
5. Run checks (see below)
6. Commit your changes using [Conventional Commits](https://www.conventionalcommits.org/)
7. Push to your fork: `git push origin my-feature`
8. Open a Pull Request against `master`

## Development

### Prerequisites

- Rust 1.75+ (install via [rustup](https://rustup.rs/))

### Running checks

Before submitting a PR, make sure all checks pass:

```bash
# Format check
cargo fmt --check

# Lint
cargo clippy -- -D warnings

# Tests
cargo test
```

### Code Style

- Follow standard Rust conventions
- Run `cargo fmt` before committing
- Address all `clippy` warnings

## Reporting Issues

- Use the [bug report template](.github/ISSUE_TEMPLATE/bug_report.md) for bugs
- Use the [feature request template](.github/ISSUE_TEMPLATE/feature_request.md) for ideas

## License

By contributing, you agree that your contributions will be licensed under the MIT License.
