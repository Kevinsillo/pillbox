<div align="center">

<img src="resources/pillbox-logo-adaptive.svg" alt="Pillbox" width="280">

# Pillbox

***The persistent memory layer for AI agents — structured, searchable, and built in Rust.***

[![Release](https://img.shields.io/github/v/release/Kevinsillo/pillbox?style=flat-square&color=e36209)](https://github.com/Kevinsillo/pillbox/releases/latest)
[![Build](https://img.shields.io/github/actions/workflow/status/Kevinsillo/pillbox/release.yml?style=flat-square)](https://github.com/Kevinsillo/pillbox/actions)
[![License](https://img.shields.io/github/license/Kevinsillo/pillbox?style=flat-square)](LICENSE)
[![Platform](https://img.shields.io/badge/platform-linux%20%7C%20macos%20%7C%20windows-blue?style=flat-square)](https://github.com/Kevinsillo/pillbox/releases/latest)
[![Node](https://img.shields.io/badge/node-%E2%89%A518-brightgreen?style=flat-square&logo=node.js)](https://nodejs.org)

</div>

Pillbox gives AI agents a structured, searchable memory that survives across sessions. Instead of relying on context windows and ad-hoc notes, the agent actively writes and retrieves knowledge as it works — decisions, bugs fixed, patterns discovered, personal conventions.

<a href="https://github.com/Kevinsillo/pillbox/stargazers">
  <img src="resources/star-banner.svg" alt="Star Pillbox on GitHub" width="720">
</a>

## Why Pillbox?

Modern AI agents are stateless by default. Every session starts from scratch. Pillbox solves this by giving the agent two types of persistent memory:

- **Pills** — project-specific knowledge. Tied to a codebase. Organized into work sessions.
- **Capsules** — personal, cross-project knowledge. Your conventions, workflow preferences, environment context, and long-term goals.

The agent writes knowledge proactively during work, and retrieves it at the start of new sessions. Over time, the agent builds a rich, searchable model of both the project and the person it's working with.

## How it works in practice

You install Pillbox once. From that point, the agent handles everything.

When you start working on a project, the agent opens a prescription and loads recent context from previous sessions — what was done, what was decided, what's pending. As work progresses, it saves relevant moments automatically: architectural decisions, bugs and their root causes, patterns established, non-obvious discoveries. When the session ends, it closes the prescription.

You never instruct the agent to remember something. It decides what matters, following the same instincts an experienced engineer would use when writing things down.

The result is an agent that feels like it was already there — one that remembers your codebase, your preferences, and your way of working.
## Core concepts

| Concept | What it is |
|---|---|
| **Bottle** | A project. Maps a directory to a database. |
| **Prescription** | A work session within a bottle. Has a title describing the task. |
| **Pill** | A piece of project knowledge saved within a prescription. |
| **Capsule** | A piece of personal knowledge — global, not tied to any project. |
## Multilingual

CLI and web interface fully localized in **6 languages**: English, Spanish, French, German, Italian, and Portuguese. Auto-detected from your system locale.

## Quick install

**Linux / macOS**

```bash
curl -fsSL https://raw.githubusercontent.com/Kevinsillo/pillbox/main/install.sh | bash
```

**Windows** (PowerShell)

```powershell
irm https://raw.githubusercontent.com/Kevinsillo/pillbox/main/install.ps1 | iex
```

This installs:
- `pillbox` binary (Rust)
- `pillbox-mcp` MCP server
- The Pillbox skill for your AI coding assistant
- The global database at `~/.pillbox/pillbox.db`

See the [installation guide](https://kevinsillo.github.io/pillbox-docs/getting-started/installation/manual/) for manual installation and platform-specific options.
## Quick start

**1. Initialize a project**

```bash
cd my-project
pillbox bottle init
```

**2. The agent takes it from here**

The installer configures the MCP server automatically. Once connected, the agent opens prescriptions, saves pills, and retrieves context on its own — guided by the Pillbox skill.

## CLI reference

<p align="center">
  <img src="resources/screenshots/demo.gif" alt="Pillbox CLI demo" width="900">
</p>

```bash
pillbox status                    # Global status
pillbox serve start [--port N]    # Start HTTP server
pillbox mcp install               # Install MCP server
pillbox skill install             # Install the Pillbox skill
pillbox bottle init               # Initialize a project
```

Full reference → [kevinsillo.github.io/pillbox-docs/reference/cli/](https://kevinsillo.github.io/pillbox-docs/reference/cli/)

## Web interface

<p align="center">
  <img src="resources/screenshots/webui-dashboard.png" alt="Pillbox WebUI dashboard" width="900">
</p>

The web dashboard gives you a visual overview of any bottle: open prescriptions, recent pills with their compound tags, activity chart, and database size. Accessible at `http://pillbox.local:4242` when the server is running.

## Architecture

Pillbox is split across four repositories that work together as a single system:

- **[pillbox](https://github.com/Kevinsillo/pillbox)** — Rust binary: CLI, HTTP server, SQLite persistence, embedded WebUI
- **[pillbox-mcp](https://github.com/Kevinsillo/pillbox-mcp)** — TypeScript MCP server, bridges the agent to the core via `pillbox exec`
- **[pillbox-skills](https://github.com/Kevinsillo/pillbox-skills)** — Skills for AI coding assistants
- **[pillbox-docs](https://github.com/Kevinsillo/pillbox-docs)** — Documentation and landing page

The MCP server communicates with the binary via `pillbox exec` — a JSON stdin/stdout dispatcher. All persistence lives in the Rust core; the MCP layer has no direct database dependency.

The web interface is built with Vite and embedded into the binary at compile time via `rust-embed`. Served at `http://pillbox.local:4242` with no external dependencies.

## License

[PolyForm Noncommercial 1.0.0](LICENSE) — free for non-commercial use.
