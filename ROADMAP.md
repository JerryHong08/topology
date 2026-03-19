# Roadmap

## Stage 1 — Graph Foundation

- [x] Scan — project files into graph JSON
  - [x] Parse directory structures
  - [x] Parse markdown headings
  - [x] Parse markdown task lists
  - [x] Layer system
  - [x] Deterministic output
- [ ] Stable ID — content-hash or path-based, consistent across scans
- [x] Query — expression-based graph traversal and filtering
  - [x] Structure queries — [roots](src/query.rs), children, descendants, ancestors
  - [x] Category queries — filter by type, status, source, label

## Stage 2 — Edge Dimensions

See [README.md](README.md) for design rationale. All edges derived from existing conventions.

- [ ] Reference edges — extract from [markdown links](https://commonmark.org) and `inline code paths`
  - [ ] Parse `[text](relative/path)` links in markdown body
  - [ ] Parse `[text](#anchor)` within-file references
  - [ ] Match `` `src/file.rs` `` inline code against known filesystem nodes
- [ ] Sequence edges — implicit ordering from sibling list items
  - [ ] Sibling tasks under same heading get `Sequence` edges
  - [ ] Sibling sections under same parent get `Sequence` edges
- [ ] Mention edges — text fragments matching known node labels or IDs
  - [ ] Scan markdown body text for node ID / label matches
  - [ ] Create weak `Mentions` edges (distinct from explicit `References`)

## Stage 3 — Query Evolution

Queries that leverage multi-dimensional edges.

- [ ] Reference traversal — `--references <ID>`, `--referenced-by <ID>`
- [ ] Sequence traversal — `--next <ID>`, `--first-in-sequence`
- [ ] Cross-edge queries — combine edge types in a single traversal
- [ ] Consolidate `status` and `context` into query subcommands

## Stage 4 — Incremental Awareness

Continuous memory over a changing codebase.

- [ ] Diff — compare two scans, output added/removed/changed nodes and edges
- [ ] Watch — re-scan on file change, emit diff stream
- [ ] Hook API — wrap watch as a callable interface for agents

## Stage 5 — Human Interface

- [ ] GUI
