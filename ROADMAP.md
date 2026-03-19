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

- [x] Reference edges — extract from [markdown links](https://commonmark.org) and `inline code paths`
  - [x] Parse `[text](relative/path)` links in markdown body
  - [x] Parse `[text](#anchor)` within-file references
  - [x] Match `` `src/file.rs` `` inline code against known filesystem nodes
- [x] Sequence edges — implicit ordering from sibling list items
  - [x] Sibling tasks under same heading get `Sequence` edges
  - [x] Sibling sections under same parent get `Sequence` edges
- [ ] Mention edges — text fragments matching known node labels or IDs
  - [ ] Scan markdown body text for node ID / label matches
  - [ ] Create weak `Mentions` edges (distinct from explicit `References`)

## Stage 3 — Query Evolution

Queries that leverage multi-dimensional edges.

- [x] Reference traversal — `--references <ID>`, `--referenced-by <ID>`
- [x] Sequence traversal — `--next <ID>`
- [ ] Cross-edge queries — combine edge types in a single traversal
- [x] Consolidate `status` and `context` into query subcommands

## Stage 4 — Incremental Awareness

Continuous memory over a changing codebase.

- [x] Diff — compare two scans, output added/removed/changed nodes and edges
- [ ] Watch — re-scan on file change, emit diff stream
- [ ] Hook API — wrap watch as a callable interface for agents

## Stage 5 — Human Interface

- [ ] GUI

## Featues

- [ ] Entropy/Garbage collect feature, archive the done tasks into colder file. can be traced back if needed?

## Open Issues

Feedback from agent usage of the topology skill.

- [x] add a option to not display hash id like '[d28e1f1]' to save context.
- [x] when there is a task and its detail.md exist at the same time, the detail.md will overwrite it, why not concanated them or the task parse first, add detail.md path as next level detailed exloration.
- [ ] `--roots` only returns filesystem root — needs layer scoping or per-file root detection
- [ ] IDs are too long — Stable ID should include short aliases or hash-based short IDs
  - [x] Short hash aliases (FNV-1a, 7 hex) — computed on-the-fly, not stored in Node
  - [x] Layered resolver — exact match → short hash → unique prefix
  - [ ] SKILL.md frontmatter generates 200+ char slug — cap slug length or use different ID strategy for frontmatter blocks
- [x] Output too verbose for agents — added `--format=compact`, `--format=ids`
- [x] No `--count` flag — added `--count`
- [x] No ID pattern search — added `id~` filter
- [x] [REFERENCE.md](.agents/skills/topology/REFERENCE.md) listed unimplemented commands — rewritten to match reality
- [x] `--` separator required for filters — replaced with `-f`/`--filter` repeated flag
- [x] No `--format=tree` — added `--format=tree` indented hierarchy view
- [ ] `topology diff` output too verbose when cache is stale — needs `--stat` summary mode (like `git diff --stat`)

### Design concerns

- [x] Scan-every-time won't scale — added `.topology.json` cache, query reads from cache if fresh
- [x] Agent ergonomics before new edge types — shipped `--format`, `--count`, `id~`
- [ ] Skill instruction too aggressive — should say when to use topology, not "always"
- [x] Graph is read-only — need `topology update <ID> status=done` to write back to markdown
- [x] Incremental awareness too far back — diff/watch should be prioritized earlier
