# CLI Reference

The `pillbox` CLI is designed primarily for human operators and for setup tasks. The agent interacts with Pillbox via the MCP server, not the CLI.

---

## `pillbox status`

Shows global status: binary path, global and local databases, active bottle, HTTP server, MCP server, and skill.

---

## Serve commands

### `pillbox serve start [--port N]`

Starts the HTTP server as a background daemon. Default port: 4242.

```bash
pillbox serve start
pillbox serve start --port 8080
```

If startup fails, the error is written to `~/.pillbox/pillbox.log`.

Publishes `pillbox._http._tcp.local.` via mDNS for local network discovery. The web UI is available at `http://localhost:<port>`.

### `pillbox serve stop`

Stops the background server.

### `pillbox serve status`

Shows whether the server is running and on which port.

---

## Bottle commands

### `pillbox bottle init`

Interactive wizard to initialize a bottle in the current directory.

**What it does:**
1. Prompts for a display name (default: directory name)
2. Prompts for scope: `local` or `global`
3. Creates the database and runs migrations
4. If local and in a git repo: offers to add `.pillbox/` to `.gitignore` (default: No)
5. If local: registers the bottle in the global DB

---

### `pillbox bottle status`

Status of the bottle in the current directory.

---

### `pillbox bottle list`

Lists all bottles registered in the global database.

---

### `pillbox bottle migrate [--reverse] [--capsules]`

Migrates a bottle between the local and global databases using upsert by `sync_id`.

```bash
pillbox bottle migrate              # local → global
pillbox bottle migrate --reverse    # global → local
```

See [docs/migration.md](migration.md) for details.

---

## Pills commands

### `pillbox pills list`

Lists all pills in the current bottle, ordered by creation date (newest first).

---

## Prescription commands

### `pillbox prescription open "<title>"`

Opens a new prescription (work session) for the current bottle.

```bash
pillbox prescription open "Implement OAuth login"
```

Fails if there is already an open prescription — close it first with `pillbox prescription close`.

---

### `pillbox prescription list [-l N]`

Lists the most recent prescriptions for the current bottle.

```bash
pillbox prescription list
pillbox prescription list -l 25
```

| Flag | Default | Description |
|---|---|---|
| `-l, --limit N` | 10 | Maximum prescriptions to show |

---

### `pillbox prescription close`

Closes the open prescription for the current bottle.

---

## MCP commands

### `pillbox mcp install`

Extracts the MCP server (embedded in the binary) to `~/.pillbox/mcp/`. Requires Node.js ≥ 18.

### `pillbox mcp uninstall`

Removes the MCP server directory.

---

## Skill commands

### `pillbox skill install`

Extracts the Claude Code skill (embedded in the binary) to `~/.claude/skills/pillbox/`.

### `pillbox skill uninstall`

Removes the skill.

---

## Language commands

### `pillbox lang`

Shows the current language and available options.

```
Idioma actual: Español (es)

es    Español  ● activo
en    English
de    Deutsch
it    Italiano
pt    Português
fr    Français
```

### `pillbox lang set <code>`

Sets the CLI language. Persisted to `~/.pillbox/lang`.

```bash
pillbox lang set en
pillbox lang set de
```

Supported codes: `es`, `en`, `de`, `it`, `pt`, `fr`.

Language detection order:
1. `~/.pillbox/lang` (set by `pillbox lang set`)
2. `PILLBOX_LANG` environment variable
3. System locale (native detection on Windows, macOS, and Linux)
4. Fallback: `es`

---

## `pillbox uninstall`

Interactive removal of Pillbox components. Prompts before each step:

- Remove the MCP server
- Remove the Claude Code skill
- Remove the global database (all memories lost)
- Remove the binary

---

## Environment variables

| Variable | Description |
|---|---|
| `PILLBOX_LANG` | Override language detection (e.g. `PILLBOX_LANG=en`) |
| `PILLBOX_VERSION` | Version to install (used by `install.sh`) |
| `PILLBOX_INSTALL_DIR` | Install directory for the binary |
| `RUST_LOG` | Log level for the server (e.g. `RUST_LOG=info pillbox serve start`) |

---

## Hidden commands

These commands are used internally and hidden from `--help`:

- `pillbox exec` — JSON stdin/stdout dispatcher used by the MCP server
- `pillbox --init-global` — creates the global database; called by `install.sh`
