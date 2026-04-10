---
name: topology
description: Navigate project roadmaps and track task progress. Use when working with roadmaps, task tracking, or when the user mentions next to do, progress, plans, or task status.
---

# Topology

## Instructions

**CRITICAL: Read [CONVENTION.md](./CONVENTION.md) before any roadmap/archive operation.**

The convention defines the structure and workflow of ROADMAP.md and ARCHIVE.md. Without understanding it, you will break the task tracking system.

Use `topo` commands for roadmap operations. Do NOT directly edit ROADMAP.md or ARCHIVE.md unless necessary.

### Before any roadmap/archive modification

1. Read CONVENTION.md to understand the rules
2. Use `topo` commands whenever possible:
   - `topo add` — add tasks
   - `topo update` — change status
   - `topo delete` — remove tasks
   - `topo section` — manage sections (add, rename, move)
   - `topo archive` — move done/dropped to ARCHIVE.md
   - `topo unarchive` — restore from ARCHIVE.md
3. If direct edit is needed, verify it follows convention:
   - ROADMAP.md: only active tasks in numbered sections + inbox
   - ARCHIVE.md: only numbered sections (NO inbox)
   - Inbox items must be promoted before archiving

### Init (first time)

If you're first running this project and use `topo` to help you track the task, check if `topo` is installed:

```bash
topo -V
```

If not, install it with:

```bash
curl -sSL https://github.com/JerryHong08/topology/releases/download/v1.0.0/topo-linux-x86_64 -o ~/.local/bin/topo && chmod +x ~/.local/bin/topo
```

Or build from source:

```bash
git clone https://github.com/JerryHong08/topology.git
cd topology
cargo install --path .
```

After that, initialize the task roadmap if it doesn't exist:

**Existing project with code:**
1. Read key files to understand the project (README, config, entry points)
2. Create `ROADMAP.md` with sections based on the project's architectural boundaries
3. Populate with detected issues: bugs, missing tests, design improvements, TODOs in code
4. Assign numeric IDs following convention
5. `mkdir -p roadmap && topo scan .`

**New/empty project:**
1. Ask the user what they want to build
2. Create `ROADMAP.md` with sections reflecting the planned architecture
3. Break the user's intent into numbered tasks and subtasks
4. `mkdir -p roadmap && topo scan .`

### Daily workflow

```bash
topo query --status                    # progress summary
topo query -f status=todo              # find next tasks
topo context <ID>                      # task details + linked docs
topo update <ID> status=in-progress    # claim task
topo update <ID> status=done           # complete task
topo archive                           # clean up done/dropped
topo archive --fix                     # auto-fix ID conflicts if any
topo scan .                            # refresh graph (when cache stale)
```

### Discussion (new task from user)

When the user proposes a new task, decide if it needs discussion:

**Quick capture** — use `topo add` with `--discuss` flag or add to inbox section manually:
```bash
topo add "Issue description" --section <N> --discuss
```

**Simple task** — add directly with `topo add`:
```bash
topo add "Task description" --section <N>
```

**Complex task** (architecture decision, multiple approaches, unclear scope):
1. Create `roadmap/<slug>.md` with discussion template
2. Discuss with the user, fill in the doc
3. Add task to ROADMAP.md with `topo add`, link the detail doc
4. `topo scan .` to refresh

### Inbox workflow (IMPORTANT)

**Inbox items NEVER go directly to archive.**

Lifecycle:
```
inbox (no ID) → promote → numbered section (with ID) → track → archive
```

When ready to work on an inbox item:
1. Move it to the appropriate numbered section
2. Assign the next available numeric ID
3. Now use `topo update <ID> status=...` to track

**Wrong:**
- Archiving inbox content directly ❌
- ARCHIVE.md has `## Open Issues` section ❌

### Task IDs

Use numeric IDs (e.g. `1.1`, `2.3.1`) or slugs:

```bash
topo context 1.3          # numeric ID (preferred)
topo context scan         # slug
```

**ID uniqueness is enforced:**
- `topo add` checks both ROADMAP.md and ARCHIVE.md when assigning IDs — no duplicates
- If a numeric ID exists in both files (e.g. from manual edit), resolver prefers ROADMAP.md
- `topo archive` detects ID conflicts with ARCHIVE.md:
  - Without `--fix`: errors with conflict details
  - With `--fix`: auto-appends `.0` suffix to conflicting archived IDs (e.g. `9.6` → `9.6.0`)

### Task statuses

| Markdown | Status | Update command |
|----------|--------|----------------|
| `- [ ]` | todo | `status=todo` |
| `- [x]` | done | `status=done` |
| `- [-]` | in-progress | `status=in-progress` |
| `- [~]` | dropped | `status=dropped` |

### Available commands

| Command | Description |
|---------|-------------|
| `topo query --status` | Progress summary |
| `topo query -f status=todo` | Find todo tasks |
| `topo query -f section=N` | Filter by section number |
| `topo query -f source=ROADMAP.md` | Filter by source file |
| `topo context <ID>` | Task details + links |
| `topo add "desc" --section <N>` | Add task |
| `topo add "desc" --section <N> --parent <ID>` | Add subtask |
| `topo update <ID> status=<S>` | Change status |
| `topo update <ID> status=<S> --link <doc>` | Change status and link detail doc |
| `topo delete <ID>` | Remove task |
| `topo archive` | Archive done/dropped |
| `topo archive --fix` | Archive with auto-fix for ID conflicts |
| `topo dedup` | Renumber tasks to ensure unique IDs |
| `topo unarchive <ID>` | Restore from archive |
| `topo scan .` | Refresh graph |
| `topo section add "Title" [--number N] [--after M]` | Add section |
| `topo section rename N "New Title"` | Rename section |
| `topo section move N --after M` | Move section order |

### API Endpoints (when running `topo serve`)

| Endpoint | Method | Description |
|----------|--------|-------------|
| `/api/graph` | GET | Get full graph data |
| `/api/layout` | GET | Get canvas layout state |
| `/api/layout` | POST | Update canvas layout |

### Section management

Manage roadmap sections (H2 headings with numeric IDs):

```bash
# Add a new section after section 7
topo section add "New Features" --after 7

# Add with specific section number
topo section add "API Design" --number 8

# Rename a section
topo section rename 8 "API Implementation"

# Move section 9 to after section 7
topo section move 9 --after 7
```

**Note:** Moving a section does NOT renumber its tasks. Task stable_ids remain unchanged to preserve references.

### Code Review with topo Task Creation

You can integrate code review with topo task tracking. Two approaches:

#### Approach 1: Manual Review + topo add

Review code yourself, then use `topo add` for findings:

```bash
# After reviewing a file and finding issues
topo add "Fix JSON.parse error handling in ws handler" --section 5
```

This is always available and works in any environment.

#### Approach 2: Custom topo-review Agent

If you frequently review code and create tasks, you can create a custom agent that does both.

**First, check if a topo-review agent exists:**
- Look in your `.agents/` directory or project's skill configuration
- If you see an agent definition that mentions topo + code review, it's available

**If no agent exists, create one:**

1. Create `.agents/` directory (or symlink to shared location)
2. Add an agent definition file:

```markdown
---
name: topo-review
description: Review code and create topo tasks for issues found
---

# topo-review Agent

## Instructions

Scan specified files for:
- Bugs and correctness issues
- Performance problems
- Best practice violations

For each real issue found, use `topo add` to create a task:
```bash
topo add "<concise description>" --section <N>
```

**IMPORTANT:** Only create tasks for significant issues:
- Production bugs or correctness risks → always create
- Real performance issues → create if measurable impact
- Code style or minor improvements → DO NOT create (these bloat roadmap)

Return a summary of findings and created task IDs.
```

3. After creating, you can use it: "Use topo-review to scan the stores directory"

**Workflow when using a review agent:**

1. Agent scans and creates tasks
2. **You MUST review the created tasks** — agent tends to over-flag
3. Delete low-priority tasks: `topo delete <ID>`
4. Keep only tasks worth tracking

See [CONVENTION.md](./CONVENTION.md) for full markdown writing rules.