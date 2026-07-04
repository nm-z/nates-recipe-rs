---
name: Read DB schema before querying
description: Always PRAGMA table_info on every table before writing queries or updates against ml_taxonomy.db or any SQLite database
type: feedback
originSessionId: 1f34447b-2e6b-4b5f-8bcf-649de253d6a4
---
Before touching any SQLite database, read the full schema first: `PRAGMA table_info(table)` on every table, check views, check constraints. Don't assume column names or types from table names alone.

**Why:** Missed the `source` column question because I'd been querying recipe_status without knowing all its columns. Also wrote updates against `implementation` and `recipe_status` without checking constraints, and initially showed incomplete data because I didn't know what columns existed.

**How to apply:** First interaction with any .db file: dump all tables, all columns, all views. Then query. Same principle as "read code before editing."
