# Selection

Goal: let agents load focused context by selecting nodes and expanding their neighborhood.

## Acceptance Criteria
- `topology select <node-id> --expand <depth>` returns a subgraph around the selected node
- `topology context "<query>"` loads the detail file for a matching task
- Falls back to scanning ROADMAP.md when no detail file exists

## Related Files
- roadmap/ — detail files loaded by context command
- src/scan/markdown.rs — parser reused for fallback task lookup
- src/context.rs — context command implementation

## Notes
- Context command slugifies the query and looks for `roadmap/<slug>.md`
- Select command extracts a subgraph within N hops of a given node
- Both commands support agent workflows: cheap tree scan first, then on-demand detail loading
