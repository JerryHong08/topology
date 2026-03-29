See [convention](.claude/skills/topology/CONVENTION.md) for markdown writing rules.

# Roadmap
## 3. Convention


## 6. Exploration

- [ ] 6.1 Watch — re-scan on file change, emit diff stream
- [x] 6.2 Web UI
  - [x] 6.2.1 HTTP API (axum) — serve graph, status, CRUD operations
  - [x] 6.2.2 WebSocket real-time updates
  - [x] 6.2.3 Delete task command
  - [x] 6.2.4 Frontend (Alpine.js) — task tree, interactions
  - [x] 6.2.5 Polish and testing
  - [ ] 6.2.6 api & cli code unify
- [ ] 6.5 独立于git的topo git like function, 用于里程碑归档以及支持branch分支进而基于不同roadmap variation版本的并行开发。

## Open Issues

- [ ] numeric id dedup
  - [x] 命令行添加删除任务触发dedup
  - [x] 手动修改原文件后也需要有一个方法dedup
