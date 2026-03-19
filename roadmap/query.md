# Query

Goal: expression-based graph queries that let agents and humans filter, search, and traverse the topology.

## Acceptance Criteria
- `topology query "<expression>"` returns matching nodes/edges as JSON
- Structure queries: navigate parent/child/sibling relationships
- Semantic queries: find nodes by content similarity
- Relation queries: follow edges (dependencies, references)
- Category queries: filter by node type, status, depth

## Related Files
- src/graph.rs — Graph types that queries operate on
- src/scan/ — scanners that produce the graph

## Notes
Structure query examples:
- `topology query "children of README.md"`
- `topology query "sections under Introduction"`

Semantic query — may use embeddings or keyword matching:
- `topology query "nodes mentioning VWAP"`

Relation query:
- `topology query "tasks depending on parser"`
- `topology query "files referencing strategy.py"`

Category query:
- `topology query "type=task status=todo"`
- `topology query "type=section depth=2"`
