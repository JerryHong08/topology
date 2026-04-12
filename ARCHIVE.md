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
- [~] 1.5 scan-every-time won't scale — solved with `.topology.json` cache
- [x] 1.6 放弃对file system乃至代码文件的nodes, edges的建模，聚焦到roadmap system的build&parse
- [~] 1.7 numeric task ID — decided yes, see convention


## 10. Web UI Fixes
- [x] 10.1 Fix WebSocket real-time updates for all operations
- [x] 10.2 Fix task status update API deadlock issue
- [x] 10.3 Fix in-progress/dropped task label duplication
  - Root cause: pulldown-cmark generates multiple Text events for `[-] 6.15 测试\n\n  测试`, causing `plain_item_text` to accumulate as `[-] 6.15 测试测试`
  - Fix: Set `plain_item_complete = true` on `Event::Start(Tag::Paragraph)` to stop accumulating description lines

## 2. Edges
- [x] 2.1 Reference edges
  - [x] 2.1.1 Parse `[text](relative/path)` links
  - [x] 2.1.2 Parse `[text](#anchor)` within-file references
  - [x] 2.1.3 Match `` `src/file.rs` `` inline code against known nodes
- [x] 2.2 Sequence edges — implicit ordering from sibling list items
- [~] 2.3 Mention edges — dropped, agent reads text directly
- [x] 2.4 Reference traversal — `--references`, `--referenced-by`
- [x] 2.5 Sequence traversal — `--next`
- [~] 2.6 Cross-edge queries — dropped, depended on 2.3


## 3. Convention
- [x] [3.1](.claude/skills/topology/CONVENTION.md) Markdown convention spec
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
- [~] 5.1 `--roots` per-file root detection — agent doesn't use it
- [~] 5.2 Cap slug length — not a real problem
- [x] 5.4 Rename binary: `topology` → `topo`
- [~] 5.5 `.topoignore` — .gitignore sufficient
- [x] 5.6 `--format tree` 不显示 numeric ID 前缀
- [x] 5.7 Inbox item 和正式 task 在 tree 输出里无法区分 — tree 输出加 checkbox marker
- [x] 5.8 `scan` 输出 87KB JSON，agent 不该直接看 — 改为默认 summary + `--json` flag
- [x] 5.9 query by default output tree format
- [~] 5.10 `diff --stat` needs more detail — what exactly should summary show?
- [x] 5.11 `topo query --status` output is json, not agent native. need to update.
- [x] 5.12 删除任务不会删除其描述
- [x] 5.13 numeric id dedup
  - [x] 5.13.1 命令行添加删除任务触发dedup
  - [x] 5.13.2 手动修改原文件后也需要有一个方法dedup


## 6. Exploration
- [x] 6.2 Web UI
  - [x] 6.2.1 HTTP API (axum) — serve graph, status, CRUD operations
  - [x] 6.2.2 WebSocket real-time updates
  - [x] 6.2.3 Delete task command
  - [x] 6.2.4 Frontend (Alpine.js) — task tree, interactions
  - [x] 6.2.5 Polish and testing
  - [x] 6.2.6 api & cli code unify
- [x] 6.3 CLI task creation — `topo add "description" --section <N>` to add tasks programmatically
- [x] 6.4 unarchive command to restore archived tasks
- [x] 6.6 Promote inbox item feature for Web UI
- [x] 6.7 Test inbox item for promote feature
- [x] 6.8 Web UI refactor to React + TypeScript + Vite
- [x] 6.13 Fix ID matching for tasks with markdown links (e.g., [3.15](url) Label)
- [x] 6.9 Replace npm with pnpm for frontend package management
- [x] 6.10 Fix detail document display in React frontend (currently blank)
- [x] 6.11 Remove unnecessary page refresh when updating/adding tasks
- [x] 6.12 Fix adding inbox task from frontend
- [x] 6.1 Watch — re-scan on file change, emit diff stream


## 7. Roadmap

- [x] 7.1 Section 管理 CLI/API
- [x] 7.2 Canvas View 原型（react-flow）
- [x] 7.3 layout.json 持久化与同步

## 8. 8. Web UI 优化

- [x] 8.1 8.1 顶部浮动状态栏 - 恢复进度显示和刷新按钮
- [x] 8.2 8.2 左侧栏与白板联动 - 点击任务聚焦到节点
- [x] 8.3 8.3 白板缩放控制 - 恢复放大/缩小/重置按钮
- [x] 8.5 8.5 对话框边界检测 - 防止创建任务对话框超出屏幕
- [x] 8.6 8.6 任务搜索/过滤 - 按状态、section、关键词筛选任务
- [x] 8.7 8.7 任务详情面板 - 点击节点展开详情（描述、子任务）
- [x] 8.8 8.8 编辑任务标题 - 双击节点直接编辑标题
- [x] 8.10 8.10 快捷键支持 - N新建、F过滤、/搜索
- [x] 8.11 8.11 批量选择 - Shift+点击选多个节点批量操作
- [x] 8.12 8.12 按 section 分区布局 - 初始布局按 section 分区域显示
- [x] 8.14 8.14 Section 分组区域 - 用带标题的半透明框显示 section 归属
- [x] 8.15 8.15 子任务连接线 - 用线连接父子任务表示层级关系
- [x] 8.16 8.16 节点防重叠 - 布局算法自动避开节点重叠
- [x] 8.17 8.17 批量选择拖动 - 框选或 Shift+点击选择多个节点

## Open Issues

- [x] `topo query -f section=N` 过滤没有效果，需要支持按 section 过滤任务
