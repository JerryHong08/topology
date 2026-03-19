# Scan

Goal: project heterogeneous file structures into a unified graph representation (Topology) as JSON.

## Acceptance Criteria
- `topology scan <path>` outputs graph JSON covering directory structures, markdown headings, and markdown task lists
- Layer filter (`--layer`) isolates filesystem or markdown nodes
- Deterministic output for identical input
- Handles nested directories and markdown files recursively

## Related Files
- src/scan/mod.rs — scanner trait and orchestration
- src/scan/directory.rs — filesystem scanner
- src/scan/markdown.rs — markdown heading + task parser
- src/graph.rs — Node/Edge/Graph types

## Notes
- Uses `ignore` crate for directory walking (respects .gitignore)
- Uses `pulldown-cmark` for markdown parsing with task list extension
- Slug-based IDs for headings and tasks, with dedup counters
- Task hierarchy expressed via indented task lists in markdown
