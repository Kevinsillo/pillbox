# HTTP API

Pillbox includes an HTTP server for local integrations, web UIs, or custom agents that prefer REST over MCP.

Start it with:

```bash
pillbox serve
# or on a custom port:
pillbox serve --port 8080
```

The server binds to `localhost` only (not exposed to the network). It publishes a mDNS service `pillbox._http._tcp.local.` so other tools on the local network can discover it.

---

## Base URL

```
http://localhost:4242/api
```

All API routes are prefixed with `/api`. The root path `/` and any path not matching a known API route serve the embedded web UI (`index.html`).

---

## Response format

All responses follow a consistent envelope:

**Success:**
```json
{ "ok": true, "data": { ... } }
```

**Created (201):**
```json
{ "ok": true, "data": { ... } }
```

**Error:**
```json
{ "ok": false, "error": "error_code", "message": "Human-readable description" }
```

---

## Pills

### `POST /pills`

Create a new pill within an open prescription.

**Body:**
```json
{
  "prescription_id": "uuid",
  "compound": "decision",
  "title": "Use JWT for session tokens",
  "content": "Full content here...",
  "dispenser": "your-agent",
  "author_name": "Ada Lovelace",
  "author_email": "ada@example.com"
}
```

`dispenser`, `author_name`, `author_email` are optional.

**Compound values:** `decision`, `architecture`, `bugfix`, `pattern`, `discovery`, `learning`, `feedback`, `prescription_summary`, `manual`

**Response:** `201 Created` with the created pill.

---

### `GET /pills/search?query=...`

Full-text search across pills. Supports prefix matching (`hex` finds `hexagonal`) and fuzzy matching for typos (`tokenizr` finds `tokenizer`). Multiple terms are ANDed together.

**Query parameters:**

| Parameter | Type | Description |
|---|---|---|
| `query` | string (required) | Search terms (space-separated). Prefix and fuzzy matching applied automatically. |
| `bottle_id` | integer | Filter by bottle |
| `compound` | string | Filter by compound type |
| `limit` | integer (1–100) | Max results (default: 20) |

**Response:**
```json
{
  "ok": true,
  "data": [
    {
      "id": 42,
      "sync_id": "uuid",
      "compound": "decision",
      "title": "Use JWT for session tokens",
      "snippet": "...highlighted match...",
      "created_at": "2026-04-14T10:00:00Z",
      "updated_at": "2026-04-14T10:00:00Z",
      "rank": -0.42,
      "prescription_id": "uuid",
      "bottle_id": 1
    }
  ]
}
```

---

### `GET /pills/:id`

Get a pill by its integer ID.

**Response:**
```json
{
  "ok": true,
  "data": {
    "id": 42,
    "sync_id": "uuid",
    "prescription_id": "uuid",
    "compound": "decision",
    "title": "Use JWT for session tokens",
    "content": "Full content...",
    "dispenser": "your-agent",
    "author_name": null,
    "author_email": null,
    "created_at": "2026-04-14T10:00:00Z",
    "updated_at": "2026-04-14T10:00:00Z"
  }
}
```

---

### `PATCH /pills/:id`

Update a pill's title and/or content.

**Body:**
```json
{
  "title": "Updated title",
  "content": "Updated content"
}
```

Both fields are optional; only provided fields are updated.

**Response:** `200 OK` with the updated pill.

---

### `DELETE /pills/:id`

Soft-delete a pill.

**Response:** `200 OK` with `{ "ok": true, "data": null }`.

---

## Capsules

### `POST /capsules`

Create a new capsule.

**Body:**
```json
{
  "compound": "convention",
  "title": "Always use conventional commits",
  "content": "All commits follow...",
  "dispenser": "your-agent"
}
```

**Compound values:** `convention`, `workflow`, `environment`, `context`, `goal`, `feedback`, `manual`

**Response:** `201 Created` with the created capsule.

---

### `GET /capsules/search?query=...`

Full-text search across capsules (global — not filtered by project). Supports prefix and fuzzy matching, same as pill search.

**Query parameters:**

| Parameter | Type | Description |
|---|---|---|
| `query` | string (required) | Search terms. Prefix and fuzzy matching applied automatically. |
| `compound` | string | Filter by compound type |
| `limit` | integer (1–100) | Max results (default: 20) |

---

### `GET /capsules/:id`

Get a capsule by ID.

### `PATCH /capsules/:id`

Update a capsule.

**Body:**
```json
{
  "title": "Updated title",
  "content": "Updated content",
  "compound": "workflow"
}
```

### `DELETE /capsules/:id`

Soft-delete a capsule.

---

## Prescriptions

### `POST /prescriptions`

Open a new work session.

**Body:**
```json
{
  "bottle_id": 1,
  "title": "Implement OAuth login"
}
```

**Response:** `201 Created` with the prescription.

**Error if already open:**
```json
{
  "ok": false,
  "error": "prescription_already_open",
  "message": "...",
  "data": {
    "id": "uuid",
    "title": "Previous task",
    "started_at": "2026-04-14T09:00:00Z",
    "pill_count": 3
  }
}
```

---

### `GET /prescriptions/:id`

Get a prescription by its UUID.

**Response:**
```json
{
  "ok": true,
  "data": {
    "id": "uuid",
    "bottle_id": 1,
    "title": "Implement OAuth login",
    "started_at": "2026-04-14T10:00:00Z",
    "ended_at": null
  }
}
```

---

### `PATCH /prescriptions/:id`

Close a prescription.

**Body:** `{}` (no body required)

**Response:** `200 OK` with the updated prescription.

---

### `DELETE /prescriptions/:id`

Discard a prescription and soft-delete all its pills in one transaction.

**Response:** `200 OK` with `{ "ok": true, "data": null }`.

---

## Bottles

### `GET /bottles`

List all registered bottles.

**Response:**
```json
{
  "ok": true,
  "data": [
    {
      "id": 1,
      "name": "my-project",
      "display_name": "My Project",
      "directory": "/home/you/my-project",
      "scope": "global"
    }
  ]
}
```

---

### `POST /bottles`

Register a new bottle.

**Body:**
```json
{
  "name": "my-project",
  "display_name": "My Project",
  "directory": "/home/you/my-project",
  "scope": "global"
}
```

**Response:** `201 Created` with the created bottle.

---

## Context

### `GET /context?bottle_id=1`

Returns recent prescriptions and pills as Markdown — same as `pill_context` in the MCP.

**Query parameters:**

| Parameter | Type | Description |
|---|---|---|
| `bottle_id` | integer (required) | The bottle to get context for |
| `prescription_limit` | integer (1–20) | Max prescriptions (default: 5) |
| `pill_limit` | integer (1–100) | Max pills per prescription (default: 20) |

**Response:**
```json
{
  "ok": true,
  "data": "## Recent context for my-project\n\n### [2026-04-14] Implement OAuth login\n\n..."
}
```

---

## Meta

### `GET /version`

Returns the running server version.

**Response:**
```json
{ "ok": true, "data": { "version": "0.4.0" } }
```

---

## mDNS discovery

When `pillbox serve` starts, it registers the service `pillbox._http._tcp.local.` on the local network. Other tools can discover it using mDNS/Bonjour without knowing the IP address.

```bash
# Discover on macOS/Linux:
dns-sd -B _http._tcp local
# or
avahi-browse -r _http._tcp
```
