# MCP Setup

Pillbox exposes its tools to AI agents via the [Model Context Protocol](https://modelcontextprotocol.io). The MCP server is a Node.js process that communicates with the `pillbox` binary via JSON over stdin/stdout.

---

## Claude Code

`pillbox mcp install` configures Claude Code automatically — it writes the `pillbox` entry into `~/.claude.json`, creating the file if needed and preserving any existing entries.

Restart Claude Code after installing. Run `/mcp` to verify the server is connected and the tools are listed.

If you prefer to configure manually, add the following to `~/.claude.json`:

```json
{
  "mcpServers": {
    "pillbox": {
      "command": "node",
      "args": ["/home/you/.pillbox/mcp/index.js"]
    }
  }
}
```

---

## Claude Desktop

Add to `~/Library/Application Support/Claude/claude_desktop_config.json` (macOS) or `%APPDATA%\Claude\claude_desktop_config.json` (Windows — not supported):

```json
{
  "mcpServers": {
    "pillbox": {
      "command": "node",
      "args": ["/home/you/.pillbox/mcp/index.js"]
    }
  }
}
```

Use the absolute path to `index.js` — `~` expansion is not guaranteed in all environments.

---

## Verifying the connection

In Claude Code, run:

```
/mcp
```

You should see `pillbox` listed with its tools:

```
pillbox (connected)
  Tools: prescription_open, prescription_close, prescription_read,
         prescription_discard, bottle_list, pill_take, pill_search,
         pill_context, pill_read, pill_revise, pill_discard,
         capsule_take, capsule_search, capsule_read, capsule_revise,
         capsule_discard
```

---

## Available tools

### Prescriptions (work sessions)

#### `prescription_open`
Opens a new work session for a bottle. Must be called before `pill_take`.

```json
{
  "bottle_id": 1,
  "title": "Implement user authentication"
}
```

Returns `{ "ok": true, "data": { "id": "uuid", "title": "...", "started_at": "..." } }`.

If a prescription is already open, returns error `prescription_already_open` with the existing prescription's data.

#### `prescription_close`
Closes the open prescription.

```json
{ "id": "prescription-uuid" }
```

#### `prescription_read`
Read a prescription by ID.

```json
{ "id": "prescription-uuid" }
```

#### `prescription_discard`
Soft-deletes a prescription and all its pills in one transaction.

```json
{ "id": "prescription-uuid" }
```

#### `bottle_list`
Lists all registered bottles.

```json
{}
```

---

### Pills (project knowledge)

#### `pill_take`
Saves a piece of project knowledge within an open prescription.

```json
{
  "prescription_id": "prescription-uuid",
  "compound": "decision",
  "title": "Use JWT for session tokens",
  "content": "We use JWT (HS256) for stateless session management. Tokens expire in 24h. Refresh tokens are stored in an httpOnly cookie.",
  "dispenser": "your-agent"
}
```

**Compounds:** `decision`, `architecture`, `bugfix`, `pattern`, `discovery`, `learning`, `feedback`, `prescription_summary`, `manual`

#### `pill_search`
Full-text search across pills.

```json
{
  "q": "JWT authentication session",
  "bottle_id": 1,
  "compound": "decision",
  "limit": 10
}
```

`bottle_id` and `compound` are optional filters.

#### `pill_context`
Returns recent prescriptions and their pills as Markdown. Call this at the start of a session to restore project context.

```json
{
  "bottle_id": 1,
  "prescription_limit": 5,
  "pill_limit": 20
}
```

#### `pill_read`
Read a pill by its integer ID.

```json
{ "id": 42 }
```

#### `pill_revise`
Update a pill's title and/or content.

```json
{
  "id": 42,
  "patch": {
    "title": "Updated title",
    "content": "Updated content"
  }
}
```

#### `pill_discard`
Soft-delete a pill.

```json
{ "id": 42 }
```

---

### Capsules (personal knowledge)

#### `capsule_take`
Saves personal, cross-project knowledge.

```json
{
  "compound": "convention",
  "title": "Always use conventional commits",
  "content": "All commits follow Conventional Commits spec. Format: type(scope): description. Types: feat, fix, chore, docs, refactor, test.",
  "dispenser": "your-agent"
}
```

**Compounds:** `convention`, `workflow`, `environment`, `context`, `goal`, `feedback`, `manual`

#### `capsule_search`
Full-text search across capsules (global — not filtered by project).

```json
{
  "query": "commit conventions",
  "compound": "convention",
  "limit": 10
}
```

#### `capsule_read`
Read a capsule by ID.

```json
{ "id": 7 }
```

#### `capsule_revise`
Update a capsule.

```json
{
  "id": 7,
  "patch": {
    "content": "Updated content",
    "compound": "workflow"
  }
}
```

#### `capsule_discard`
Soft-delete a capsule.

```json
{ "id": 7 }
```

---

## Response format

All tools return a consistent JSON envelope:

**Success:**
```json
{ "ok": true, "data": { ... } }
```

**Error:**
```json
{ "ok": false, "error": "error_code", "message": "Human-readable description" }
```

**Notable error codes:**
- `prescription_already_open` — a prescription is already open for this bottle. `data` contains the existing prescription.
- `not_found` — the requested resource does not exist.
- `validation_error` — invalid input.

---

## How it works

The MCP server does not access the database directly. It communicates with the `pillbox` binary via `pillbox exec`:

1. The MCP server receives a tool call from the agent.
2. It spawns `pillbox exec` as a subprocess.
3. It writes a JSON payload to stdin: `{ "op": "pill_take", "params": { ... } }`.
4. It reads the JSON response from stdout.
5. It returns the response to the agent.

This means the MCP server is stateless and has no SQLite dependency. All persistence is handled by the Rust core.
