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

## Windows builds

Native Windows binaries are built with the MSVC toolchain. Run these from the `pillbox/` directory:

```bash
make build-win-msvc-x64     # x86_64-pc-windows-msvc
make build-win-msvc-arm64   # aarch64-pc-windows-msvc
```

Prerequisites:

- A Windows host with the MSVC toolchain (Visual Studio Build Tools)
- The matching rustup targets installed: `rustup target add x86_64-pc-windows-msvc` and `rustup target add aarch64-pc-windows-msvc`

These targets are excluded from `build-all`, which only covers the Linux/CI builds. They will not run in CI.

### Releasing Windows assets

The release upload is manual. After building, rename the produced `pillbox.exe` and upload it as a GitHub Release asset using the names `install.ps1` expects:

- `core/target/x86_64-pc-windows-msvc/release/pillbox.exe` → `pillbox-windows-x86_64.exe`
- `core/target/aarch64-pc-windows-msvc/release/pillbox.exe` → `pillbox-windows-aarch64.exe`

### Verifying Windows-only behavior

The CI gate runs on Linux only, so Windows-specific behavior is not covered automatically. Verify it manually on a Windows host before release:

- The `install.ps1` user-scope installer (downloads the binary, updates user `PATH`)
- The elevation guard on `pillbox serve install` and `pillbox serve uninstall` (must require an elevated PowerShell)

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
