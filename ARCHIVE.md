# Archive

## 1. Core
- [x] [1.1](roadmap/scan.md) Scan — parse markdown into graph JSON
  - [x] 1.1.1 Parse markdown headings
  - [x] 1.1.2 Parse markdown task lists
  - [x] 1.1.3 Deterministic output
- [x] 1.2 Query — expression-based traversal and filtering
  - [x] 1.2.1 Structure queries — roots, children, descendants, ancestors
  - [x] 1.2.2 Category queries — filter by type, status, source, label
  - [x] 1.2.3 Consolidate status and context into query subcommands
- [x] [1.3](roadmap/context-link-design.md) Stable ID — numeric task ID system
  - [x] 1.3.1 Parse numeric ID prefix from task text
  - [x] 1.3.2 Use numeric ID as stable_id, derive slug from remaining label
  - [x] 1.3.3 Resolver supports numeric ID lookup
- [x] 1.4 Diff — compare two scans



## 2. Edges
- [x] 2.1 Reference edges
  - [x] 2.1.1 Parse `[text](relative/path)` links
  - [x] 2.1.2 Parse `[text](#anchor)` within-file references
  - [x] 2.1.3 Match `` `src/file.rs` `` inline code against known nodes
- [x] 2.2 Sequence edges — implicit ordering from sibling list items
- [~] 2.3 Mention edges — dropped, agent reads text directly
- [x] 2.4 Reference traversal — `--references`, `--referenced-by`
- [x] 2.5 Sequence traversal — `--next`
- [~] 2.3 Mention edges — dropped, agent reads text directly
- [~] 2.6 Cross-edge queries — dropped, depended on 2.3



## 3. Convention
- [x] [3.1](.claude/skills/topology/CONVENTION.md) Markdown convention spec


## 3. [Convention & Abstraction](roadmap/abstration_manifesto.md)
- [x] 3.2 Parser: numeric task ID extraction — depends on 1.3
- [x] 3.3 Parser: `[-]` in-progress status
- [x] 3.4 Parser: `[~]` dropped status
- [x] 3.5 Skill redesign — update skill instructions to match convention
- [x] 3.6 Archive workflow — move done/dropped tasks to `roadmap/archive.md`
- [~] [3.7](roadmap/abstration_discussion.md) 五原语 DSL — 方向不对



## 4. Incremental
- [~] 4.2 Hook API — use case unclear, dropped
- [~] 4.3 Archive / entropy — done as `topo archive` (3.6)



## 5. Tooling
- [x] 5.4 Rename binary: `topology` → `topo`
- [~] 5.1 `--roots` per-file root detection — agent doesn't use it
- [~] 5.2 Cap slug length — not a real problem
- [~] 5.5 `.topoignore` — .gitignore sufficient



## Design Concerns
- [~] numeric task ID — decided yes, see [convention](.claude/skills/topology/CONVENTION.md)
- [~] scan-every-time won't scale — solved with `.topology.json` cache
- [x] 放弃对file system乃至代码文件的nodes, egdes的建模，聚焦到roadmap system的build&parse



## Open Issues
- [x] `--format tree` 不显示 numeric ID 前缀
- [x] Inbox item 和正式 task 在 tree 输出里无法区分 — tree 输出加 checkbox marker
- [x] `scan` 输出 87KB JSON，agent 不该直接看 — 改为默认 summary + `--json` flag
- [x] query by default output tree format
- [~] `diff --stat` needs more detail — what exactly should summary show?
