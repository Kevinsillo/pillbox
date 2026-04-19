# Concepts

Pillbox organizes knowledge around four entities: **bottles**, **prescriptions**, **pills**, and **capsules**. Understanding how they relate to each other is the foundation for using Pillbox effectively.

---

## Bottles

A **bottle** represents a project. It maps a directory on disk to a Pillbox database.

```
bottles
├── id           integer  — internal ID
├── name         text     — slug (auto-generated from directory name, immutable)
├── display_name text     — human-readable name (chosen during init)
├── directory    text     — absolute path to the project directory
└── scope        text     — "local" or "global"
```

**Scope** determines which database the bottle lives in:

- `local` — `.pillbox/pillbox.db` inside the project directory. Good for project-specific work that shouldn't mix with other projects.
- `global` — `~/.pillbox/pillbox.db`. Good for projects where you want all knowledge in one place, or for knowledge you want to share across machines.

A bottle's `name` is generated automatically from the directory name (normalized to a URL-safe slug). The `display_name` is what you choose when running `pillbox bottle init`.

---

## Prescriptions

A **prescription** is a work session within a bottle. It has a title that describes what you're working on.

```
prescriptions
├── id         text      — UUID
├── bottle_id  integer   — the bottle this session belongs to
├── title      text      — task or feature description (required, set at open time)
├── started_at datetime
└── ended_at   datetime  — null while open
```

**Rules:**
- There can only be one open prescription per bottle at a time.
- The title must be set when opening — describe the task before starting.
- Pills cannot be saved without an open prescription.
- Closing a prescription marks it as ended; pills are preserved.
- Discarding a prescription soft-deletes it and all its pills in one transaction.

The prescription is the unit of work. When the agent starts a new task, it opens a prescription. When the task is done (or paused), it closes it with a summary pill.

---

## Pills

A **pill** is a piece of project-specific knowledge saved within a prescription.

```
pills
├── id              integer  — internal ID
├── sync_id         text     — UUID for cross-database sync
├── prescription_id text     — the session this pill belongs to
├── compound        text     — knowledge type (see below)
├── title           text     — short description (1–200 chars)
├── content         text     — full content (Markdown)
├── dispenser       text     — who saved it (e.g., "claude-sonnet-4")
├── author_name     text     — git author name, if relevant
├── author_email    text     — git author email, if relevant
└── created_at / updated_at / deleted_at
```

### Pill compounds

The `compound` field classifies the type of knowledge:

| Compound | Use for |
|---|---|
| `decision` | Architecture, design, or approach decisions with rationale |
| `architecture` | System structure, module layout, data flow |
| `bugfix` | Bug description, root cause, and fix |
| `pattern` | Reusable patterns or conventions found/established in this project |
| `discovery` | Non-obvious findings about the codebase, dependencies, or environment |
| `learning` | Something that failed and what was learned from it |
| `feedback` | User feedback on agent behavior or approach |
| `prescription_summary` | End-of-session summary (one per prescription, saved on close) |
| `manual` | Anything that doesn't fit the above |

Pills are searchable via FTS5 full-text search across `title` and `content`. The search engine supports prefix matching (`hex` finds `hexagonal`) and fuzzy matching to tolerate typos, using Jaro-Winkler similarity with parallel vocab scanning via rayon.

---

## Capsules

A **capsule** is personal, cross-project knowledge. It belongs to the user, not to any project.

```
capsules
├── id         integer  — internal ID
├── sync_id    text     — UUID for sync
├── compound   text     — knowledge type (see below)
├── title      text
├── content    text
├── dispenser  text
└── created_at / updated_at / deleted_at
```

Capsules have no `bottle_id` — they are always global.

### Capsule compounds

| Compound | Use for |
|---|---|
| `convention` | Code style, naming, formatting preferences |
| `workflow` | How you like to work: PR process, review style, task breakdown |
| `environment` | Machine setup, installed tools, shell config, paths |
| `context` | Personal context: current role, focus area, constraints |
| `goal` | Long-term objectives, priorities, or targets |
| `feedback` | Feedback on agent behavior that should persist across all projects |
| `manual` | Anything else |

---

## How they fit together

```
User
└── Capsules (global personal knowledge)

Project directory
└── Bottle (name, display_name, scope, directory)
    └── Prescriptions (work sessions)
        └── Pills (knowledge saved during the session)
```

**Typical agent workflow:**

1. **Session start** — call `pill_context` to retrieve recent prescriptions and pills. Call `capsule_search` with relevant terms to load personal conventions.
2. **During work** — call `pill_take` to save decisions, bugs fixed, discoveries.
3. **Session end** — call `pill_take` with `compound: "prescription_summary"` to summarize the session, then `prescription_close`.

---

## Soft deletes

All entities use soft deletes (`deleted_at` timestamp). Nothing is permanently removed from the database. Discarded pills, capsules, prescriptions, and bottles remain in the DB with `deleted_at` set and are excluded from all queries by default.
