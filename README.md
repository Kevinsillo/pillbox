# Pillbox

> Persistent knowledge memory for AI agents.

Pillbox gives AI agents a structured, searchable memory that survives across sessions. Instead of relying on context windows and ad-hoc notes, the agent actively writes and retrieves knowledge as it works — decisions, bugs fixed, patterns discovered, personal conventions.

---

## Why Pillbox?

Modern AI agents are stateless by default. Every session starts from scratch. Pillbox solves this by giving the agent two types of persistent memory:

- **Pills** — project-specific knowledge. Tied to a codebase. Organized into work sessions.
- **Capsules** — personal, cross-project knowledge. Your conventions, workflow preferences, environment context, and long-term goals.

The agent writes knowledge proactively during work, and retrieves it at the start of new sessions. Over time, the agent builds a rich, searchable model of both the project and the person it's working with.

---

## Core concepts

| Concept | What it is |
|---|---|
| **Bottle** | A project. Maps a directory to a database. |
| **Prescription** | A work session within a bottle. Has a title describing the task. |
| **Pill** | A piece of project knowledge saved within a prescription. |
| **Capsule** | A piece of personal knowledge — global, not tied to any project. |

See [docs/concepts.md](docs/concepts.md) for the full data model.

---

## Quick install

```bash
curl -fsSL https://get.pillbox.dev | bash
```

This installs:
- `pillbox` binary (Rust)
- `pillbox-mcp` Node.js MCP server
- The Claude Code skill at `~/.claude/skills/pillbox/`
- The global database at `~/.pillbox/pillbox.db`

See [docs/installation.md](docs/installation.md) for manual installation and platform-specific options.

---

## Quick start

**1. Initialize a project**

```bash
cd my-project
pillbox bottle init
```

The wizard asks for a display name and whether to use a local or global database.

**2. Configure the MCP in Claude Code**

Add to your `~/.claude.json`:

```json
{
  "mcpServers": {
    "pillbox": {
      "command": "node",
      "args": ["~/.pillbox/mcp/dist/index.js"]
    }
  }
}
```

See [docs/mcp-setup.md](docs/mcp-setup.md) for full configuration details.

**3. The agent takes it from here**

Once the MCP is connected, the agent will open prescriptions, save pills, and retrieve context automatically — guided by the [Pillbox skill](~/.claude/skills/pillbox/SKILL.md).

---

## CLI reference

```
pillbox status                         Global status: DBs, active bottle, server, MCP, skill
pillbox serve start [--port N] [-d]    Start the HTTP server (--daemon for background)
pillbox serve stop                     Stop the background server
pillbox serve status                   Show server status

pillbox bottle init                    Initialize a bottle (interactive wizard)
pillbox bottle status                  Status of the current bottle
pillbox bottle list                    List all registered bottles
pillbox bottle migrate [--reverse]     Migrate local ↔ global

pillbox pills list                     List pills in the current bottle

pillbox prescription open <title>      Open a work session
pillbox prescription list [-l N]       List prescriptions
pillbox prescription close             Close the open prescription

pillbox mcp install                    Install the MCP server
pillbox mcp uninstall                  Remove the MCP server

pillbox skill install                  Install the Claude Code skill
pillbox skill uninstall                Remove the Claude Code skill

pillbox lang                           Show current language and available options
pillbox lang set <code>                Set CLI language (es, en, de, it, pt, fr)

pillbox uninstall                      Remove Pillbox components
```

See [docs/cli.md](docs/cli.md) for the full CLI reference.

---

## MCP tools

The MCP server exposes these tools to the agent:

**Session management**
- `prescription_open` — start a work session
- `prescription_close` — end a work session
- `prescription_read` — read session details
- `prescription_discard` — discard a session and all its pills
- `bottle_list` — list registered projects

**Project knowledge (pills)**
- `pill_take` — save a piece of knowledge
- `pill_find` — full-text search
- `pill_context` — retrieve recent session context (use at session start)
- `pill_read` — read a pill by ID
- `pill_revise` — update a pill
- `pill_discard` — soft-delete a pill

**Personal knowledge (capsules)**
- `capsule_take` — save personal/cross-project knowledge
- `capsule_find` — full-text search
- `capsule_read` — read a capsule by ID
- `capsule_revise` — update a capsule
- `capsule_discard` — soft-delete a capsule

See [docs/api.md](docs/api.md) for the HTTP API and [docs/mcp-setup.md](docs/mcp-setup.md) for MCP configuration.

---

## Architecture

```
pillbox/
├── core/          Rust binary + library (SQLite, CLI, HTTP server, embedded WebUI)
├── webui/         Vue 3 web interface (built and embedded into the binary)
├── mcp/           TypeScript MCP server (bridges Claude ↔ core via exec)
├── docs/          Extended documentation
└── install.sh     One-line installer
```

The MCP server communicates with the Rust binary via `pillbox exec` — a JSON stdin/stdout dispatcher. This means the MCP layer has no database dependency; all persistence lives in the Rust core.

The web interface is built with Vite and embedded into the binary at compile time via `rust-embed`. In production it is served at `http://localhost:4242` with no external dependencies.

---

## Inspiration

Pillbox is inspired by the idea that AI agents should accumulate knowledge the same way experienced engineers do — by writing things down, organizing them, and referencing them later. The pharmaceutical metaphor (bottles, pills, prescriptions) reflects a deliberate, measured approach to knowledge management: you don't dump everything at once, you prescribe exactly what's needed.

---

## Built with

**Rust core**

| Crate | Purpose |
|---|---|
| [rusqlite](https://github.com/rusqlite/rusqlite) | SQLite (bundled), FTS5 full-text search |
| [axum](https://github.com/tokio-rs/axum) | HTTP server |
| [tokio](https://tokio.rs) | Async runtime |
| [clap](https://github.com/clap-rs/clap) | CLI argument parsing |
| [inquire](https://github.com/mikaelmello/inquire) | Interactive prompts |
| [indicatif](https://github.com/console-rs/indicatif) | Progress spinners |
| [tabled](https://github.com/zhiburt/tabled) | Terminal tables |
| [owo-colors](https://github.com/jam1garner/owo-colors) | Terminal colors |
| [rust-embed](https://github.com/pyrossh/rust-embed) | Embed WebUI into binary |
| [rust-i18n](https://github.com/longbridgeapp/rust-i18n) | Internationalization (6 languages) |
| [sys-locale](https://github.com/1Password/sys-locale) | Native locale detection (Windows/macOS/Linux) |
| [mdns-sd](https://github.com/keepsimple1/mdns-sd) | mDNS local network discovery |
| [serde](https://serde.rs) | Serialization |
| [anyhow](https://github.com/dtolnay/anyhow) | Error handling |

**Web UI**

| Package | Purpose |
|---|---|
| [Vue 3](https://vuejs.org) | UI framework (Composition API) |
| [Vite](https://vitejs.dev) | Build tool |
| [Tailwind CSS](https://tailwindcss.com) | Styling |
| [Pinia](https://pinia.vuejs.org) | State management |
| [Vue Router](https://router.vuejs.org) | Client-side routing |
| [vue-i18n](https://vue-i18n.intlify.dev) | Internationalization |
| [unplugin-icons](https://github.com/unplugin/unplugin-icons) | Bundled icon components (Lucide, flag icons) |
| [marked](https://marked.js.org) | Markdown rendering |

**MCP server**

| Package | Purpose |
|---|---|
| [@modelcontextprotocol/sdk](https://github.com/modelcontextprotocol/typescript-sdk) | MCP protocol |
| [zod](https://zod.dev) | Schema validation |

---

## License

PolyForm Noncommercial 1.0.0
