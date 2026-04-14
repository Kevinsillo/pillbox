# Migration

Pillbox supports two database scopes: **local** (`.pillbox/pillbox.db` in the project directory) and **global** (`~/.pillbox/pillbox.db`). Migration moves a bottle's knowledge between them.

---

## When to migrate

**Local → Global:** You started a project with a local database and now want to consolidate everything into the global DB — for example, before archiving the project, or to make the knowledge searchable alongside other projects.

**Global → Local:** You want to work offline or in an isolated environment with a self-contained local copy.

---

## How it works

Migration uses **upsert by `sync_id`** — a UUID assigned to each pill and capsule when created. This means:

- If a record doesn't exist in the destination, it's inserted.
- If it already exists, it's updated only if `updated_at` in the source is more recent.
- Soft-deleted records (`deleted_at IS NOT NULL`) are migrated as-is — soft deletes propagate.
- The migration is idempotent: running it twice is safe.

The full migration chain for a bottle:
1. Bottle (upsert by `name`)
2. Prescriptions (upsert by `id` UUID)
3. Pills (upsert by `sync_id`)
4. Capsules (optional, global — upsert by `sync_id`)

---

## CLI usage

### Local → Global (default)

```bash
cd my-project
pillbox bottle migrate
```

Both databases must exist. The bottle must be registered in the local DB.

### Global → Local

```bash
cd my-project
pillbox bottle migrate --reverse
```

### Include capsules

Capsules are global and have no bottle. Use `--capsules` to include them in the migration (copies all capsules from source DB to destination DB):

```bash
pillbox bottle migrate --capsules
pillbox bottle migrate --reverse --capsules
```

### Output

```
Migrando bottle 'my-project' (local → global)...
✓ Bottles:        1
✓ Prescripciones: 5
✓ Pills:          23
```

---

## Requirements

- The `pillbox` binary must be in `$PATH`.
- The global DB must exist (`~/.pillbox/pillbox.db`). Created by `pillbox --init-global` or `install.sh`.
- For local → global: the local DB (`.pillbox/pillbox.db`) must exist in the current directory.
- For global → local: the local DB must already exist (run `pillbox bottle init` first if needed).
- The bottle must be registered in the source database.

---

## Conflict resolution

Conflicts are resolved by `updated_at` timestamp:

- The more recently updated record wins.
- This means you can freely edit in either database and then merge — the latest edit wins.
- There is no manual conflict resolution step.

**Note:** This strategy assumes clocks are reasonably in sync between machines. If you're migrating between machines with clock drift, the `updated_at` comparison may produce unexpected results.

---

## Scope after migration

After migration, the `scope` field on the bottle in the destination DB reflects the original value from the source. If you want to change a bottle's scope permanently, edit the database directly:

```bash
sqlite3 ~/.pillbox/pillbox.db \
  "UPDATE bottles SET scope = 'global' WHERE name = 'my-project'"
```

---

## Example: consolidating multiple projects

If you have three projects with local databases and want to consolidate:

```bash
cd ~/projects/project-a && pillbox bottle migrate
cd ~/projects/project-b && pillbox bottle migrate
cd ~/projects/project-c && pillbox bottle migrate --capsules
```

After this, all pills are in `~/.pillbox/pillbox.db` and searchable together with `pill_find`.
