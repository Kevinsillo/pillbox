# Contributing to Pillbox

Thanks for your interest in contributing.

## Before opening a PR

**Open an issue first.** This project has a defined roadmap and not every change or feature fits its direction. Opening an issue before writing code avoids wasted effort if the proposal doesn't align with where the project is going.

PRs opened without a prior issue will be closed.

## Related repos

- [pillbox-mcp](https://github.com/Kevinsillo/pillbox-mcp) — MCP server (TypeScript)
- [pillbox-skills](https://github.com/Kevinsillo/pillbox-skills) — Community skills
- [pillbox-docs](https://github.com/Kevinsillo/pillbox-docs) — Documentation and landing

## Requirements

- [Rust](https://rustup.rs) (stable)
- [Node.js](https://nodejs.org) >= 20
- [pnpm](https://pnpm.io)
- [cargo-watch](https://crates.io/crates/cargo-watch) (optional, for `make dev`)

## Getting started

```bash
git clone https://github.com/Kevinsillo/pillbox
cd pillbox
make build-full
make install
```

## Development

```bash
make dev         # Rust core in watch mode
make dev-webui   # Vue dev server
```

## Tests

```bash
make test    # Rust tests
make check   # test + lint + fmt-check
```

## Commit convention

This project follows [Conventional Commits](https://www.conventionalcommits.org):

```
feat(core): add fuzzy search
fix(db): handle PRAGMA in transactions
refactor(cli): split main.rs into cmd/ module
docs: update installation guide
```

## Opening a PR

1. Open an issue and wait for feedback before writing code
2. Fork the repo and create a branch from `main`
3. Keep commits focused and conventional
4. Run `make check` before pushing
5. Reference the issue in the PR description
