---
name: topology
description: Navigate project roadmaps and track task progress. Use when working with roadmaps, task tracking, or when the user mentions next to do, progress, plans, or task status.
---

# Topology

## Instructions

Use the `topo` command to navigate the project roadmap and track task progress.


### Init (first time)

if you're first running this project and use `topo` to help you track the task, you need to check if `topo` is installed and working:

```bash
topo -V
```

if not, install it with:

```bash
curl -sSL https://github.com/JerryHong08/topology/releases/download/v1.0.0/topo-linux-x86_64 -o ~/.local/bin/topo && chmod +x ~/.local/bin/topo
```

Or build from source:

```bash
git clone https://github.com/JerryHong08/topology.git
cd topology
cargo install --path .
```

after that, you need to initialize the task roadmap if it doesn't exist:

If the project has no `ROADMAP.md`, initialize one:

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
topo query -f status=todo              # find next tasks
topo query --status                    # progress summary
topo context <ID>                      # task details + linked docs
topo update <ID> status=in-progress    # claim task
topo update <ID> status=done           # complete task
topo archive                           # clean up done/dropped
topo scan .                            # refresh graph (when cache stale)
```

### Discussion (new task from user)

When the user proposes a new task, decide if it needs discussion:

**Simple task** — add directly to ROADMAP.md with next available numeric ID.

**Complex task** (architecture decision, multiple approaches, unclear scope):
1. Create `roadmap/<slug>.md` with discussion template:
   - **Context**: why this task, current project state
   - **Analysis**: related code, history (check ARCHIVE.md for similar dropped tasks), risks
   - **Decision**: chosen approach and why
   - **Rejected**: alternatives considered
   - **Plan**: implementation steps
2. Discuss with the user, fill in the doc
3. Add task to ROADMAP.md with numeric ID, link the detail doc
4. `topo scan .` to refresh

### Task IDs

Use numeric IDs (e.g. `1.1`, `2.3.1`) or slugs:

```bash
topo context 1.3          # numeric ID (preferred)
topo context scan         # slug
```

### Task statuses

| Markdown | Status | Update command |
|----------|--------|----------------|
| `- [ ]` | todo | `status=todo` |
| `- [x]` | done | `status=done` |
| `- [-]` | in-progress | `status=in-progress` |
| `- [~]` | dropped | `status=dropped` |

See [convention](./CONVENTION.md) for full markdown writing rules.
