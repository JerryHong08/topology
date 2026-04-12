# Topo Robustness & Agent Workflow Analysis

Source: deep analysis of `jerry_trader` project — a production system driven by agents using `/topology` skill.

## Core Finding

Convention compliance is unreliable when enforced by text alone. The toolchain itself must detect and prevent violations.类比: linter > coding style guide.

## Issues Found (by severity)

### 1. Scan Noise Explosion (Critical)

**Symptom**: `topo scan .` produces 6324 nodes in jerry_trader, but only ~300 are from ROADMAP.md + roadmap/.

- 3072 nodes from `node_modules/` (pnpm symlinks bypass gitignore)
- ~600 from `discard/` (gitignored but still scanned)
- ~200 from `.claude/worktrees/` (Claude Code worktree copies)

**Root cause**: `ignore::WalkBuilder` doesn't respect `.gitignore` when pnpm symlink structure resolves through `.pnpm/` directory. `.claude/worktrees/` is a full copy that also gets scanned.

**Impact**:
- 4.4MB `.topology.json` cache
- `topo query` returns irrelevant results
- `topo context 3.15` → ambiguous match (ARCHIVE.md + worktree copy)

**Fix**: Explicit scan scope — either whitelist (only scan ROADMAP.md, ARCHIVE.md, roadmap/, .agents/skills/) or blacklist (always exclude node_modules/, .claude/worktrees/, .git/).

### 2. ID Conflict: ARCHIVE.md vs ROADMAP.md (Critical)

**Symptom**: `topo archive --dry-run` fails with ID conflict error.

**Root cause**: Agent archived section 9 tasks (9.1=Backtest data models), then later assigned same IDs to new tasks (9.1=Data CLI). Convention says "IDs only increment, never reuse" but agent forgot what was archived.

**Why agent violated**: No tool-level enforcement. Agent has no memory of archived IDs across sessions.

**Fix**: `topo scan` should detect and warn about ID conflicts between ROADMAP.md and ARCHIVE.md.

### 3. Done Tasks Piling Up in ROADMAP.md (Medium)

**Symptom**: Section 9 has 19/20 done, Section 10 has 3/3 done, but never archived.

**Root cause**: No clear trigger for when to archive. SKILL.md says "daily workflow" but doesn't define when.

**Fix**: `topo query --status` should prompt "Section X complete — run `topo archive`" when a section hits 100%.

### 4. Free-form Text in ROADMAP.md (Medium)

**Symptom**: Bug analysis notes written as plain indented text under tasks (not as subtasks or linked detail docs).

```
- [x] 5.24 RankList virtual scroll
  staticProfileCache从不清理，导致 localStorage 超限...
  line 538 的 seriesInitializedRef.current.clear()...
```

**Root cause**: Convention doesn't explicitly forbid notes in ROADMAP. Agent treats ROADMAP as notebook instead of index.

**Fix**: `topo scan` could lint for non-checkbox text under tasks.

### 5. Inbox Items Without Checkbox Format (Minor)

**Symptom**: Open Issues items written as plain text, not `- [ ]` format. Invisible to `topo query --status`.

**Root cause**: Convention says inbox tasks don't need numeric IDs, but doesn't require checkbox format.

**Fix**: Either convention update or parser tolerance.

### 6. Section Description Text (Minor)

Section headings followed by descriptive paragraphs. Not harmful but invisible to topo context.

## Root Cause Matrix

| Deviation | Root Cause | Fix via topo? | Fix via convention? |
|-----------|-----------|--------------|-------------------|
| ID reuse | Agent forgets archived IDs | scan-time conflict detection | text rules unreliable |
| No archive | No trigger condition | status auto-prompt | add trigger rules |
| Notes in ROADMAP | No format check | scan-time lint | add format rules |
| Inbox no checkbox | Convention unclear | parser tolerance | clarify format |
| node_modules noise | Walker ignores gitignore | explicit scan scope | N/A |
| worktree duplicates | Claude worktree scanned | exclude .claude/worktrees | N/A |

## Proposed Tasks

- **P0**: Scan scope control — explicit file whitelist or directory blacklist
- **P1**: ID conflict detection at scan time
- **P2**: Smart archive prompts in status output
- **P3**: ROADMAP format lint (non-checkbox text, free-form notes)
- **P4**: Convention updates (inbox format, notes policy)
