# Roadmap

## 6. Exploration

- [ ] 6.5 Independent git-like function for milestone archiving and branch-based parallel development on different roadmap variations

## 7. Roadmap

- [ ] 7.4 Web UI — ship task to agent work pipeline & roadmap

## 8. Web UI

- [ ] 8.18 Document workspace editing — edit and save documents
- [ ] 8.19 List view toggle — canvas/list mode switch
- [ ] 8.20 Create task at viewport center — not fixed position
- [ ] 8.21 Gantt chart view — timeline-based task progress and dependencies
- [ ] 8.22 ACT-R decision simulation model — cognitive architecture for task decision support

## 9. Task.md Convention

- [ ] 9.1 Task.md convention design — subtasks in detail docs instead of ROADMAP.md indentation
- [ ] 9.2 Task.md parser — recognize subtasks in docs and link to parent task

## 11. Robustness

Agent workflow reliability improvements based on jerry_trader production analysis. See [roadmap/topo-robustness-analysis.md](roadmap/topo-robustness-analysis.md).

- [ ] 11.1 Scan scope control — explicit whitelist/blacklist, stop scanning node_modules/, .claude/worktrees/, .git/
- [ ] 11.2 ID conflict detection — warn at scan time when ROADMAP.md and ARCHIVE.md share IDs
- [ ] 11.3 Smart archive prompts — `topo query --status` suggests archive when section is 100% done
- [ ] 11.4 ROADMAP format lint — detect non-checkbox inbox items, free-form text under tasks
- [ ] 11.5 Convention updates — inbox format (must use `- [ ]`), notes policy (all analysis in detail docs)

## Open Issues


## Design Concerns
