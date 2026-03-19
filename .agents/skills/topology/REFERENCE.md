# Topology Reference

## Global Flags

- `--hash` — show short hash IDs in output (hidden by default to save context). Works with all commands.

```bash
topology scan .                        # no hashes in output
topology scan . --hash                 # nodes include "short" field
topology query -f type=task --hash     # hashes in query output
topology context scan --hash           # hashes in context output
```

## Node Resolution

Any command that accepts a node ID uses the layered resolver. You don't need to type full IDs.

Resolution order: exact match → short hash → full ID prefix → slug exact → slug prefix.

```bash
topology context ROADMAP.md#scan-project-files-into-graph-json   # exact
topology context d28e1f1                                          # short hash
topology context ROADMAP.md#scan                                  # prefix
topology context scan                                             # slug (portion after #)
topology context stage-1                                          # slug prefix
```

## CLI

```bash
topology scan .                        # full graph (filesystem + markdown), writes .topology.json cache
topology scan . --layer=markdown       # markdown layer only
topology status                        # roadmap progress summary
topology context scan                  # load node context (any node type)
topology context scan --json           # context as JSON
topology update "task-id" status=done  # update task status
```

## Context

Shows the graph neighborhood of any node — ancestors, children, references, referenced-by, and detail doc path.

```bash
topology context scan                  # by slug
topology context stage-1               # by slug prefix
topology context d28e1f1               # by short hash
topology context src/main.rs           # file nodes work too
topology context scan --json           # structured JSON output
topology context scan --hash           # include short hash IDs
```

Output sections (only non-empty sections are shown):
- Header: label, source, type, status
- Ancestors: containment chain up to root
- Subtasks/Children: direct children via Contains edges
- References: outgoing reference edges
- Referenced by: incoming reference edges
- Detail: path to `roadmap/*.md` detail doc (if found)

## Query

Use `-f` (or `--filter`) for filter expressions. Flags and filters can be mixed freely.

```bash
# basic filters
topology query -f type=task -f status=todo
topology query -f "label~scan"
topology query -f "id~stage-2"

# traversal
topology query --roots
topology query --children "ROADMAP.md#stage-1-graph-foundation"
topology query --descendants "ROADMAP.md#stage-2-edge-dimensions"
topology query --ancestors "ROADMAP.md#some-task-id"
topology query --next "ROADMAP.md#some-task-id"

# output formats
topology query --format=compact -f type=task -f status=todo
topology query --format=ids -f type=task
topology query --format=tree --descendants "ROADMAP.md#stage-2-edge-dimensions"
topology query --count -f type=task -f status=todo
topology query --status

# combined
topology query --format=compact --descendants "ROADMAP.md#stage-2-edge-dimensions" -f type=task
```

### Flags

- `--format=json` (default) full graph JSON
- `--format=compact` tab-separated id and label per line
- `--format=ids` one id per line
- `--format=tree` indented hierarchy view (follows Contains edges)
- `--count` print only the number of matching nodes
- `--status` show stage/task progress summary (like `topology status`)
- `--roots` nodes with no incoming edges
- `--children <ID>` direct children of a node
- `--descendants <ID>` all transitive children
- `--ancestors <ID>` path from node up to root
- `--next <ID>` next sibling in document order (sequence edge)
- `--references <ID>` nodes that ID links to (outgoing reference edges)
- `--referenced-by <ID>` nodes that link to ID (incoming reference edges)
- `--path <PATH>` directory to scan (default `.`)

### Filter expressions

- `type=task` match node kind (directory, file, section, task)
- `status=todo` match metadata field
- `source=markdown` match source (filesystem, markdown)
- `label~keyword` label contains keyword (case-insensitive)
- `id~fragment` id contains fragment (case-insensitive)

## Edge Types

Edges connect nodes in the graph. Each edge has `source`, `target`, and `type`.

- `contains` — structural parent→child (directory→file, section→task, heading→subheading)
- `references` — cross-cutting links extracted from markdown `[text](path)`, `[text](#anchor)`, and inline code paths like `` `src/file.rs` ``
- `sequence` — document-order edges between siblings under the same parent (task→next task, section→next section)

Reference edges are resolved against known node IDs — only links pointing to actual nodes in the graph produce edges. External URLs and broken links are silently skipped.

```bash
# list all reference edges
topology scan . | jq '.edges[] | select(.type == "references")'

# compact source → target view
topology scan . | jq -r '.edges[] | select(.type == "references") | "\(.source) → \(.target)"'

# sequence edges between siblings
topology scan . | jq -r '.edges[] | select(.type == "sequence") | "\(.source) → \(.target)"'

# what does a node reference?
topology query --references "ROADMAP.md#stage-2-edge-dimensions"

# what references a node?
topology query --referenced-by "README.md"
```
