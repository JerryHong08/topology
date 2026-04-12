# Topology Markdown Convention

**CRITICAL: Read this document before modifying ROADMAP.md or ARCHIVE.md.**

Topology builds a project graph by scanning markdown. This document defines the markdown writing conventions so both humans and agents can efficiently draw and read the map.

## Core principles

1. **ROADMAP.md is a snapshot of current state**, not a history log. Keep it readable and scannable.
2. **roadmap/ directory holds expanded details**, linked back from ROADMAP.md.
3. **Standard markdown only** — no custom syntax. Humans can read it directly, topology can parse it.
4. **Use `topo` commands for operations** — do not directly edit ROADMAP.md/ARCHIVE.md unless absolutely necessary.
5. **ROADMAP.md is an index, not a notebook** — all analysis, bug details, and design notes go in `roadmap/<slug>.md` detail docs. Only task titles belong in ROADMAP.md.

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

## ARCHIVE.md structure rules

**ARCHIVE.md only contains numbered sections. No inbox sections allowed.**

```
# Archive

## 1. Core
- [x] 1.1 Done task

## 2. Edges
- [~] 2.3 Dropped task
```

**Wrong:**
```markdown
# Archive

## 1. Core
...

## Open Issues              ← ❌ Inbox section in archive
- [x] Some issue
```

**Why:** Inbox items were never tracked. They must be promoted to numbered sections before archiving.

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
- Do NOT repeat the number in the heading text (`## 8. Web UI` not `## 8. 8. Web UI`)
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

### Inbox (non-numeric sections)

Unnumbered headings after numbered sections collect unprocessed items:

```markdown
## Open Issues              ← inbox: specific problems found during use
## Design Concerns          ← inbox: design decisions needing discussion
```

**Rules:**
- Inbox headings have no numeric prefix
- Tasks in inbox sections don't get numeric IDs either
- **Inbox items MUST use `- [ ]` checkbox format** — plain text lines are invisible to `topo query`
- **No number = unprocessed** — this is a signal by itself

**Inbox workflow (IMPORTANT):**

```
┌─────────┐    ┌──────────────────┐    ┌─────────┐    ┌──────────┐
│  Inbox  │ →  │ Numbered Section │ →  │  Track  │ →  │  Archive │
│ (no ID) │    │  (assign ID)     │    │ (topo)  │    │  (topo)  │
└─────────┘    └──────────────────┘    └─────────┘    └──────────┘
```

**RULE: Inbox content cannot be archived directly.**

The correct lifecycle:
1. **Capture** — quickly jot ideas/issues in inbox (no numeric ID needed)
2. **Promote** — move to numbered section and assign ID
3. **Track** — now it can be updated via `topo update <ID> status=...`
4. **Archive** — only after it has a numeric ID

**Wrong:**
- ARCHIVE.md contains `## Open Issues` section ❌
- Inbox items deleted or archived without promotion ❌
- Directly editing ROADMAP.md/ARCHIVE.md instead of using `topo` ❌

**Right:**
- ARCHIVE.md only has numbered sections ✓
- Inbox items promoted before archiving ✓
- Use `topo add`, `topo update`, `topo archive` for operations ✓

Only tasks in numbered sections have stable IDs. Inbox tasks must be promoted before they can be tracked via CLI.

## Numeric task IDs

### Format

```
<section>.<sequence>            top-level task: 1.1, 1.2, 2.1
<section>.<sequence>.<sub>      nested task: 1.1.1, 1.1.2
```

### Rules

- Numeric ID goes after the checkbox, before the task description
- Do NOT repeat the numeric ID in the description text (`- [ ] 8.4 Task` not `- [ ] 8.4 8.4 Task`)
- Numeric IDs are **stable identifiers** — once assigned, they don't change even if the title is edited
- IDs only increment, never reuse — deleted/archived tasks keep their numbers
- **Always check ARCHIVE.md before assigning a new ID** to avoid conflicts with archived tasks
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
| todo | `- [ ] x.x` | `status=todo` |
| in-progress | `- [-] x.x` | `status=in-progress` |
| done | `- [x] x.x` | `status=done` |
| dropped | `- [~] x.x` | `status=dropped` |

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

After discussion, add task to ROADMAP.md with `topo add`, assign ID, link detail doc.

### Archiving

Use `topo archive` to move done/dropped tasks from ROADMAP.md to `ARCHIVE.md`:

```bash
topo archive --dry-run    # preview
topo archive              # execute
```

- Archived tasks keep their IDs and section grouping. They remain queryable in the graph.
- **Archive promptly** — when `topo query --status` shows a section is fully done/dropped, run `topo archive`. Don't let done tasks pile up.

## Agent workflow

**Always use `topo` commands for roadmap operations:**

| Operation | Command |
|-----------|---------|
| View tasks | `topo query --status` |
| Task details | `topo context <ID>` |
| Add task | `topo add "description" --section <N>` |
| Update status | `topo update <ID> status=<status>` |
| Delete task | `topo delete <ID>` |
| Archive | `topo archive` |
| Unarchive | `topo unarchive <ID>` |
| Refresh graph | `topo scan .` |

**Do NOT directly edit ROADMAP.md or ARCHIVE.md** unless:
- Manual cleanup is required (e.g., fixing formatting issues)
- Promoting inbox items to numbered sections
- The operation is not supported by `topo` CLI

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
5. Add to ROADMAP.md: `topo add "description" --section <N> --discuss`
6. Link detail doc if created

### Draw the map (simple tasks)

```bash
topo add "Task description" --section 1    # add to section 1
topo scan .                                # refresh graph
```

### Execute

```bash
topo update 1.1 status=in-progress    # start
topo update 1.1 status=done           # complete
topo archive                          # clean up
```

### Promoting inbox items

When an inbox item is ready to be worked on:

1. Find the appropriate numbered section
2. Assign the next available ID in that section
3. Move the task from inbox to that section with the ID
4. Now use `topo update <ID> status=in-progress` to track it
5. Run `topo scan .` to refresh

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
- [ ] `--roots` needs per-file root detection
```