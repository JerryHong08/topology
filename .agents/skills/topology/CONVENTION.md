# Topology Markdown Convention

Topology builds a project graph by scanning markdown. This document defines the markdown writing conventions so both humans and agents can efficiently draw and read the map.

## Core principles

1. **ROADMAP.md is a snapshot of current state**, not a history log. Keep it readable and scannable.
2. **roadmap/ directory holds expanded details**, linked back from ROADMAP.md.
3. **Standard markdown only** — no custom syntax. Humans can read it directly, topology can parse it.

## File structure

```
ROADMAP.md              ← top-level map (hot, active tasks)
ARCHIVE.md              ← done/dropped tasks (cold)
roadmap/
  <topic>.md            ← detail docs for specific tasks or discussions
```

- ROADMAP.md only keeps **active** tasks (todo / in-progress)
- Done or dropped tasks are archived to `ARCHIVE.md` via `topo archive`
- Tasks needing expanded discussion get a detail file under `roadmap/`, linked from ROADMAP.md

## Heading structure

```markdown
# Roadmap

## 1. Module/domain name               ← H2 = top-level group, numbered
- [ ] 1.1 task description             ← task with numeric ID
  - [ ] 1.1.1 subtask                  ← nested task

## 2. Another module
- [ ] 2.1 task description
```

**Rules:**
- H2 headings use numeric prefixes as section numbers (`## 1.`, `## 2.`)
- H2 is the primary grouping level, divided by the project's architectural boundaries
- H3 only when sub-grouping is necessary
- No H4+ — too much nesting makes the map hard to read

### Section organization

Sections are defined by the agent during bootstrap based on project structure. Different projects have different boundaries:

- CLI tool: `Core`, `Edges`, `Tooling`
- Web app: `Frontend`, `Backend`, `Infrastructure`

**Agent responsibilities for sections:**
- Initialize sections based on project structure during bootstrap
- When human proposes new work, agent decides which section it belongs to and assigns the ID
- Once assigned, numbers never change — deleted sections don't get their numbers reused

### Inbox

Unnumbered headings after numbered sections collect unprocessed items:

```markdown
## Open Issues              ← inbox: specific problems found during use
## Design Concerns          ← inbox: design decisions needing discussion
```

**Rules:**
- Inbox headings have no numeric prefix
- Tasks in inbox sections don't get numeric IDs either
- **No number = unprocessed** — this is a signal by itself
- Agent periodically triages inbox: clear items → move to numbered section with ID; decided items → mark `[~]` dropped

## Numeric task IDs

### Format

```
<section>.<sequence>            top-level task: 1.1, 1.2, 2.1
<section>.<sequence>.<sub>      nested task: 1.1.1, 1.1.2
```

### Rules

- Numeric ID goes after the checkbox, before the task description
- Numeric IDs are **stable identifiers** — once assigned, they don't change even if the title is edited
- IDs only increment, never reuse — deleted/archived tasks keep their numbers
- Agent is responsible for assigning IDs; humans don't need to worry about it

### Parser extraction

```
Markdown source:
  - [ ] 1.1 Scan — project files into graph JSON

Parser extracts:
  stable_id: "1.1"
  label: "Scan — project files into graph JSON"
  slug: "scan-project-files-into-graph-json"  (auto-generated from label)

Usage:
  topo context 1.1        ← locate by numeric ID
  topo context scan       ← locate by slug
```

## Task syntax

### Basic format

```markdown
- [ ] 1.1 Task description
- [x] 1.2 Completed task
```

### Status markers

| Status | Markdown | topo update |
|--------|----------|-------------|
| todo | `- [ ]` | `status=todo` |
| in-progress | `- [-]` | `status=in-progress` |
| done | `- [x]` | `status=done` |
| dropped | `- [~]` | `status=dropped` |

### Linked docs

Tasks link to detail docs via markdown links. `topo context` shows linked .md file paths with size:

```markdown
- [ ] [1.1](roadmap/scan.md) Scan — parse markdown into graph
- [ ] 3.1 Spec — [convention](docs/conv.md), [examples](docs/ex.md)
```

```
❯ topo context 1.1
# Scan — parse markdown into graph
task | todo

## Links
  roadmap/scan.md (20 lines, ~116 tokens)
```

### Dependencies

Reference numeric IDs directly in task text:

```markdown
- [ ] 2.1 Cross-edge queries — depends on 1.3
```

## Task lifecycle

```
idea → discuss → todo → in-progress → done → archived
                                     → dropped → archived
```

### Discussion (significant tasks)

Important or complex tasks should go through a discussion phase before execution. Record the discussion in `roadmap/<slug>.md`:

```markdown
# Task: <title>

## Context
Why this task exists. Current project state, user requirements.

## Analysis
- Related code/design in the current project
- Similar past decisions (check ARCHIVE.md)
- Risks and considerations

## Decision
Conclusion from discussion. Which approach was chosen, and why.

## Rejected
Alternatives considered but discarded, and why.

## Plan
Concrete implementation steps.
```

**When discussion is needed:**
- Architecture decisions or multiple viable approaches
- Changes affecting multiple modules or files
- User requirements that are unclear and need refinement
- Potential conflicts with existing design

**When discussion is not needed:**
- Simple bug fixes that can be described in one line
- Small, well-defined feature additions
- Documentation-only updates

After discussion, update ROADMAP.md (assign ID, link detail doc) and the task enters todo status.

### Archiving

Use `topo archive` to move done/dropped tasks from ROADMAP.md to `ARCHIVE.md`:

```bash
topo archive --dry-run    # preview
topo archive              # execute
```

Archived tasks keep their IDs and section grouping. They remain queryable in the graph.

## Agent workflow

### Read the map

```bash
topo query -f type=task -f status=todo    # find tasks
topo context 1.1                          # read task details
```

### Discussion (when new requirements come in)

1. Evaluate: is it worth doing? Does it overlap with existing tasks?
2. Check history: any similar dropped tasks in `ARCHIVE.md`? Why were they dropped?
3. Analyze impact: which modules are affected? What are the risks?
4. If discussion is needed, write Context / Analysis / Decision in `roadmap/<slug>.md`
5. Add conclusion to ROADMAP.md, assign ID, link detail doc
6. `topo scan .` to refresh graph

### Draw the map (simple tasks)

1. Decide which section the task belongs to
2. Assign the next available numeric ID
3. Link detail doc if needed
4. Write to ROADMAP.md
5. `topo scan .` to refresh graph

### Execute

```bash
topo update 1.1 status=in-progress    # start
topo update 1.1 status=done           # complete
topo archive                          # clean up
```

## Full example

```markdown
# Roadmap

## 1. Core
- [ ] [1.1](roadmap/scan.md) Scan — parse markdown into graph
  - [ ] 1.1.1 Parse headings
  - [ ] 1.1.2 Parse task lists
- [-] 1.2 Query — expression-based traversal

## 2. Tooling
- [ ] 2.1 Watch — re-scan on file change
- [~] 2.2 GUI — deferred

## Open Issues
- `--roots` needs per-file root detection
```
