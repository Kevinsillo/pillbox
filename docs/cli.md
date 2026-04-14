# CLI Reference

The `pillbox` CLI is designed primarily for human operators and for setup tasks. The agent interacts with Pillbox via the MCP server, not the CLI.

---

## Global commands

### `pillbox list`

Lists all bottles registered in the global database.

```
Bottles registrados (2)
────────────────────────────────────────────────────────────────────────
#    Nombre                 Display                Directorio
────────────────────────────────────────────────────────────────────────
1    my-project             My Project             /home/you/my-project
2    api-server             API Server             /home/you/api-server
────────────────────────────────────────────────────────────────────────
```

---

### `pillbox status`

Shows the active database status: schema version, pill count, and capsule count.

```
DB: /home/you/.pillbox/pillbox.db
Schema:   v1
Pills:    47
Capsules: 12
```

Resolves the active database in this order:
1. `.pillbox/pillbox.db` in the current directory (local)
2. `~/.pillbox/pillbox.db` (global)

---

### `pillbox doctor`

System diagnostics. Checks the binary, global DB, local DB, and current bottle.

```
Pillbox Doctor
════════════════════════════════════════════════════════════════════════

Binario      /home/you/.local/bin/pillbox

DB global    /home/you/.pillbox/pillbox.db
             ✓ Schema v1  —  2 bottles  —  47 pills  —  12 capsules

DB local     .pillbox/pillbox.db  (no existe en este directorio)

Bottle       my-project — "My Project" — global
             Prescripción abierta: "Implement OAuth login"
```

---

### `pillbox serve [--port PORT]`

Starts the HTTP server. Default port: 4242.

```bash
pillbox serve
pillbox serve --port 8080
```

Publishes `pillbox._http._tcp.local.` via mDNS. Stops gracefully on Ctrl+C.

---

## Bottle commands

### `pillbox bottle init`

Interactive wizard to initialize a bottle in the current directory.

```
Inicializando bottle en /home/you/my-project...

? ¿Cómo quieres llamar a este proyecto? › my-project
? ¿Dónde guardar las memories? › local  — .pillbox/pillbox.db (solo este proyecto)
? ¿Añadir .pillbox/ a .gitignore? › Yes

✓ Bottle 'my-project' creado.

  Slug:    my-project
  Display: My Project
  DB:      /home/you/my-project/.pillbox/pillbox.db

✓ Registrado en DB global.

Listo. Usa 'pillbox bottle status' para ver el estado.
```

**What it does:**
1. Prompts for a display name (default: directory name)
2. Prompts for scope: `local` or `global`
3. Creates the database and runs migrations
4. If local and in a git repo: offers to add `.pillbox/` to `.gitignore`
5. If local: registers the bottle in the global DB (fire-and-forget)

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

### `pillbox bottle list [-l N]`

Lists the most recent pills in the current bottle.

```bash
pillbox bottle list
pillbox bottle list -l 50
```

```
Pills de 'my-project' (últimas 20)
────────────────────────────────────────────────────────────────────────
#     Compound         Título
────────────────────────────────────────────────────────────────────────
1     decision         Use JWT for session tokens
2     bugfix           Fix token expiry not checked on refresh
3     architecture     Auth module structure
...
────────────────────────────────────────────────────────────────────────
```

**Options:**

| Flag | Default | Description |
|---|---|---|
| `-l, --limit N` | 20 | Maximum pills to show |

---

### `pillbox bottle migrate [--reverse] [--capsules]`

Migrates a bottle between the local and global databases using upsert by `sync_id`.

```bash
# local → global (default)
pillbox bottle migrate

# global → local
pillbox bottle migrate --reverse

# include capsules in the migration
pillbox bottle migrate --capsules
```

```
Migrando bottle 'my-project' (local → global)...
✓ Bottles:        1
✓ Prescripciones: 5
✓ Pills:          23
```

See [docs/migration.md](migration.md) for details.

---

## Prescription commands

### `pillbox prescription open "<title>"`

Opens a new prescription (work session) for the current bottle.

```bash
pillbox prescription open "Implement OAuth login"
```

```
✓ Prescripción abierta: "Implement OAuth login"
  ID: a1b2c3d4-...
```

**Error if already open:**

```
Error: Ya hay una prescripción abierta: "Previous task" (3 pills, id=deadbeef).
Ciérrala con 'pillbox prescription close' antes de abrir una nueva.
```

---

### `pillbox prescription list [-l N]`

Lists the most recent prescriptions for the current bottle.

```
Prescriptions de 'my-project' (últimas 10)
────────────────────────────────────────────────────────────────────────
ID         Título                                   Estado
────────────────────────────────────────────────────────────────────────
a1b2c3d4   Implement OAuth login                    abierta
e5f6a7b8   Fix token expiry bug                     cerrada
...
────────────────────────────────────────────────────────────────────────
```

**Options:**

| Flag | Default | Description |
|---|---|---|
| `-l, --limit N` | 10 | Maximum prescriptions to show |

---

### `pillbox prescription close`

Closes the open prescription for the current bottle.

```
✓ Prescripción cerrada: "Implement OAuth login"
```

**Error if none open:**

```
Error: No hay ninguna prescripción abierta para el bottle 'my-project'.
```

---

## Environment variables

| Variable | Description |
|---|---|
| `PILLBOX_VERSION` | Version to install (used by `install.sh`) |
| `PILLBOX_INSTALL_DIR` | Install directory for the binary |
| `RUST_LOG` | Log level for the server (e.g. `RUST_LOG=info pillbox serve`) |

---

## Hidden commands

These commands are used internally and hidden from `--help`:

- `pillbox exec` — JSON stdin/stdout dispatcher used by the MCP server
- `pillbox --init-global` — creates the global database; called by `install.sh`
