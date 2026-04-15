---
name: taxonomy
description: Query the ML taxonomy database (docs/ml_taxonomy.db). Use when the user asks about coverage, what's implemented, what's missing, crate status, method counts, or any question about the algorithm zoo.
autoInvoke: true
---

# ML Taxonomy Database

The project has an SQLite database at `docs/ml_taxonomy.db` tracking every ML algorithm, its crate availability, and implementation status.

## Schema

```
category (id, name, description)
subcategory (id, category_id, name, description)
method (id, subcategory_id, name, aliases, pipeline_stage, data_type, supervised, complexity, year_introduced, seminal_paper, notes)
tag (id, name)
method_tag (method_id, tag_id)
rust_crate (id, name, version, maintained, in_cargo, url, notes)
implementation (id, method_id, crate_id, status[available|partial|planned|missing|workaround], quality[production|usable|experimental|broken|unknown], notes)
recipe_status (id, method_id, implemented, tested, lua_exposed, module_path, source[hand-rolled|crate], notes)
```

## Views

- `v_full_taxonomy` — category/subcategory/method joined
- `v_missing` — unimplemented methods
- `v_crate_coverage` — per-crate available/partial/missing counts
- `v_coverage_by_stage` — implementation % by pipeline_stage

## How to Use

Answer taxonomy questions by running `sqlite3 -header -column docs/ml_taxonomy.db "..."` and presenting the table output directly. Keep queries focused — the user wants tables, not prose.

Common queries:

```bash
# Coverage summary
sqlite3 -header -column docs/ml_taxonomy.db "SELECT (SELECT count(*) FROM method) as total, (SELECT count(*) FROM recipe_status WHERE implemented=1) as done, (SELECT count(*) FROM recipe_status WHERE source='crate') as crate, (SELECT count(*) FROM recipe_status WHERE source='hand-rolled') as handrolled;"

# What's missing in a category
sqlite3 -header -column docs/ml_taxonomy.db "SELECT m.name FROM method m JOIN subcategory s ON m.subcategory_id=s.id JOIN category c ON s.category_id=c.id LEFT JOIN recipe_status rs ON m.id=rs.method_id WHERE c.name='Regressors' AND (rs.implemented IS NULL OR rs.implemented=0) ORDER BY m.name;"

# Coverage by category
sqlite3 -header -column docs/ml_taxonomy.db "SELECT c.name, count(*) as total, count(CASE WHEN rs.implemented=1 THEN 1 END) as done, count(CASE WHEN rs.source='crate' THEN 1 END) as crate FROM category c JOIN subcategory s ON s.category_id=c.id JOIN method m ON m.subcategory_id=s.id LEFT JOIN recipe_status rs ON rs.method_id=m.id GROUP BY c.name ORDER BY total DESC;"

# Crate availability for a method
sqlite3 -header -column docs/ml_taxonomy.db "SELECT rc.name, i.status, i.quality, i.notes FROM implementation i JOIN rust_crate rc ON rc.id=i.crate_id WHERE i.method_id=(SELECT id FROM method WHERE name LIKE '%RandomForest%' LIMIT 1);"

# All implemented items
sqlite3 -header -column docs/ml_taxonomy.db "SELECT m.name, rs.source, rs.module_path FROM recipe_status rs JOIN method m ON m.id=rs.method_id WHERE rs.implemented=1 ORDER BY rs.source, m.name;"
```

## Rules

- Always use `sqlite3 -header -column` for readable output
- Present query results directly as tables — no rewording into prose
- For updates, always read current state first (`SELECT` before `UPDATE`)
- When adding new implementations, set the `source` column (`hand-rolled` or `crate`)
