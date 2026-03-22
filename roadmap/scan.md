# Scan

Goal: parse markdown files into a graph representation (Topology) as JSON.

## Acceptance Criteria
- `topo scan <path>` outputs graph JSON covering markdown headings and task lists
- Deterministic output for identical input
- Handles nested markdown files recursively

## Related Files
- src/scan/mod.rs — scanner orchestration, caching, reference resolution
- src/scan/markdown.rs — markdown heading + task parser
- src/graph.rs — Node/Edge/Graph types

## Notes
- Uses `ignore` crate for directory walking (respects .gitignore)
- Uses `pulldown-cmark` for markdown parsing with task list extension
- Slug-based IDs for headings and tasks, with dedup counters
- Task hierarchy expressed via indented task lists in markdown
- Numeric task ID extraction (e.g. "1.1 Scan..." → stable_id="1.1")
