# CLI Reference

The `pillbox` CLI is designed primarily for human operators and for setup tasks. The agent interacts with Pillbox via the MCP server, not the CLI.

---

## `pillbox status`

Shows global status: binary path, global and local databases, active bottle, HTTP server, MCP server, and skill.

```
Pillbox Status

Binario        /home/you/.local/bin/pillbox
Global Bottle  /home/you/.pillbox/pillbox.db
               ● Schema: v1  Bottles: 2  Capsules: 12
Local Bottle   /home/you/my-project/.pillbox/pillbox.db
               ● Pills: 23  Prescriptions: 5
               Rx:  "Implement OAuth login"
Servidor Web   ● en ejecución — http://localhost:4242
MCP            ● /home/you/.pillbox/mcp/dist/index.js
Skill          ● /home/you/.claude/skills/pillbox/SKILL.md
```

---

## Serve commands

### `pillbox serve start [--port N] [--daemon]`

Starts the HTTP server. Default port: 4242.

```bash
pillbox serve start
pillbox serve start --port 8080
pillbox serve start --daemon       # background process
```

Publishes `pillbox._http._tcp.local.` via mDNS for local network discovery. The web UI is available at `http://localhost:<port>`.

### `pillbox serve stop`

Stops the background server.

### `pillbox serve status`

Shows whether the server is running and on which port.

---

## Bottle commands

### `pillbox bottle init`

Interactive wizard to initialize a bottle in the current directory.

```
Inicializando bottle en /home/you/my-project...

? ¿Cómo quieres llamar a este proyecto? › my-project
? ¿Dónde guardar las memories? › local  — .pillbox/pillbox.db (solo este proyecto)
? ¿Añadir .pillbox/ a .gitignore? › No

● Bottle 'my-project' creado.

  Slug:    my-project
  Display: My Project
  DB:      /home/you/my-project/.pillbox/pillbox.db

● Registrado en DB global.

Listo. Usa 'pillbox bottle status' para ver el estado.
```

**What it does:**
1. Prompts for a display name (default: directory name)
2. Prompts for scope: `local` or `global`
3. Creates the database and runs migrations
4. If local and in a git repo: offers to add `.pillbox/` to `.gitignore` (default: No)
5. If local: registers the bottle in the global DB

---

### `pillbox bottle status`

Status of the bottle in the current directory.

```
Bottle:  my-project — "My Project"
Scope:   local
Dir:     /home/you/my-project
Pills:   23
Rx:      "Implement OAuth login" (abierta, id=a1b2c3d4)
```

---

### `pillbox bottle list`

Lists all bottles registered in the global database.

```
Bottles registrados (2)

#    Nombre          Display         Directorio
──────────────────────────────────────────────────────
1    my-project      My Project      /home/you/my-project
2    api-server      API Server      /home/you/api-server
```

---

### `pillbox bottle migrate [--reverse] [--capsules]`

Migrates a bottle between the local and global databases using upsert by `sync_id`.

```bash
pillbox bottle migrate              # local → global
pillbox bottle migrate --reverse    # global → local
pillbox bottle migrate --capsules   # include global capsules
```

See [docs/migration.md](migration.md) for details.

---

## Pills commands

### `pillbox pills list`

Lists all pills in the current bottle, ordered by creation date (newest first).

```
Pills de 'my-project'

#    Compound        Título
──────────────────────────────────────────────────────────────────
1    decision        Use JWT for session tokens
2    bugfix          Fix token expiry not checked on refresh
3    architecture    Auth module structure
```

---

## Prescription commands

### `pillbox prescription open "<title>"`

Opens a new prescription (work session) for the current bottle.

```bash
pillbox prescription open "Implement OAuth login"
```

```
● Prescripción abierta: "Implement OAuth login"
  ID: a1b2c3d4-...
```

Error if already open:

```
Error: Ya hay una prescripción abierta: "Previous task" (3 pills, id=deadbeef).
Ciérrala con 'pillbox prescription close' antes de abrir una nueva.
```

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

```
● Prescripción cerrada: "Implement OAuth login"
```

---

## MCP commands

### `pillbox mcp install`

Downloads and installs the MCP server to `~/.pillbox/mcp/`. Requires Node.js ≥ 18.

### `pillbox mcp uninstall`

Removes the MCP server directory.

---

## Skill commands

### `pillbox skill install`

Downloads and installs the Claude Code skill to `~/.claude/skills/pillbox/SKILL.md`.

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
