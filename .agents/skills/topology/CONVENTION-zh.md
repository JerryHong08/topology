# Topology Markdown Convention

Topology 通过 scan markdown 来构建 project graph。本文档定义 markdown 的书写约定，使 human 和 agent 都能高效地绘制和阅读地图。

## 核心原则

1. **ROADMAP.md 是当前状态的快照**，不是历史日志。保持可读、可扫。
2. **roadmap/ 目录存放展开的细节**，通过 link 连接回 ROADMAP.md。
3. **标准 markdown**，不引入非标准语法。人能直接读，topology 能 parse。

## 文件结构

```
ROADMAP.md              ← 顶层地图（hot，活跃的 task）
ARCHIVE.md              ← 完成/放弃的 task（cold）
roadmap/
  <topic>.md            ← 某个 task 或领域的展开详情
```

- ROADMAP.md 只保留 **活跃的** task（todo / in-progress）
- 完成或放弃的 task 通过 `topo archive` 归档到 `ARCHIVE.md`
- 需要展开讨论的 task 在 `roadmap/` 下建详情文件，从 ROADMAP.md 中 link 过去

## Heading 结构

```markdown
# Roadmap

## 1. 模块/功能域名               ← H2 = 顶层分组，带编号
- [ ] 1.1 task 描述               ← task，带数字 ID
  - [ ] 1.1.1 子 task             ← 嵌套 task

## 2. 另一个模块
- [ ] 2.1 task 描述
```

**规则**：
- H2 带数字前缀作为 section 编号（`## 1.`, `## 2.`）
- H2 作为主要分组粒度，按项目的 architectural boundary 划分
- H3 仅在必要时用于子分组
- 不用 H4+，层级太深地图反而难读

### Section 组织

Section 的划分由 agent 在 bootstrap 时根据项目结构决定。不同项目 boundary 不同：

- CLI 工具可能按功能分：`Core`, `Edges`, `Tooling`
- Web app 可能按层分：`Frontend`, `Backend`, `Infrastructure`

**Agent 的 section 管理职责**：
- Bootstrap 时根据项目结构初始化 section
- Human 提新需求时，agent 决定放哪个 section、分配编号
- 编号一旦分配不变，section 删除后编号不复用

### Inbox

编号 section 之后放无编号的 inbox heading，用于收集未处理的 item。

```markdown
## Open Issues              ← inbox：使用中发现的具体问题
## Design Concerns          ← inbox：需要讨论的设计决策
```

**规则**：
- Inbox heading 不带数字编号
- Inbox 中的 task item 也不分配数字 ID
- **无编号 = 未处理**，这本身是一个信号
- Agent 定期分拣 inbox：明确的 → 进编号 section 并分配 ID；已决策的 → 标记 `[~]` dropped

## Task 数字 ID

### 格式

```
<section>.<sequence>            一级 task：1.1, 1.2, 2.1
<section>.<sequence>.<sub>      嵌套 task：1.1.1, 1.1.2
```

### 规则

- 数字 ID 写在 checkbox 之后、task 描述之前
- 数字 ID 是**稳定标识**——一旦分配，不随标题修改而变
- 编号只增不减，删除/归档后编号不复用
- Agent 负责分配编号，human 不需要关心

### Parser 提取

```
Markdown source:
  - [ ] 1.1 Scan — project files into graph JSON

Parser 提取:
  stable_id: "1.1"
  label: "Scan — project files into graph JSON"
  slug: "scan-project-files-into-graph-json"  （从 label 自动生成）

使用:
  topo context 1.1        ← 用数字 ID 定位
  topo context scan       ← 用 slug 定位
```

## Task 写法

### 基本格式

```markdown
- [ ] 1.1 Task 描述
- [x] 1.2 已完成的 task
```

### 状态标记

| 状态 | Markdown | topo update |
|------|----------|-------------|
| todo | `- [ ]` | `status=todo` |
| in-progress | `- [-]` | `status=in-progress` |
| done | `- [x]` | `status=done` |
| dropped | `- [~]` | `status=dropped` |

### 关联文档（Links）

Task 通过 markdown link 关联详情文档。`topo context` 会显示关联的 .md 文件路径和大小：

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

### 依赖关系

用文本直接引用数字 ID：

```markdown
- [ ] 2.1 Cross-edge queries — depends on 1.3
```

## Task 生命周期

```
idea → discuss → todo → in-progress → done → archived
                                     → dropped → archived
```

### Discussion（重要 task）

重要或复杂的 task 在进入执行前，应该经过 discussion 阶段。在 `roadmap/<slug>.md` 中记录讨论过程：

```markdown
# Task: <title>

## Context
为什么要做这个。当前项目状态、用户需求背景。

## Analysis
- 项目现阶段的相关代码/设计
- 历史上做过的类似决策（查 ARCHIVE.md）
- 风险和注意事项

## Decision
讨论后的结论。选了什么方案，为什么。

## Rejected
考虑过但放弃的方案，以及原因。

## Plan
具体的实施步骤。
```

**什么时候需要 discussion：**
- 涉及架构决策或多个可行方案
- 影响多个模块或文件
- 用户提出的需求不够明确，需要细化
- 与现有设计有潜在冲突

**什么时候不需要：**
- 一句话能说清楚的 bug fix
- 明确的小功能添加
- 纯文档更新

Discussion 完成后，将结论更新到 ROADMAP.md（分配 ID、link detail doc），task 进入 todo 状态。

### 归档

通过 `topo archive` 将完成/放弃的 task 从 ROADMAP.md 移到 `ARCHIVE.md`：

```bash
topo archive --dry-run    # 预览
topo archive              # 执行
```

归档保留编号和分组。归档后的 task 仍在 graph 中可查询。

## Agent 工作流

### 读图

```bash
topo query -f type=task -f status=todo    # 找活
topo context 1.1                          # 看任务详情
```

### Discussion（新需求进来时）

1. 评估需求：值不值得做？和现有 task 有无重叠？
2. 查历史：`ARCHIVE.md` 中有没有类似的被 drop 的 task？原因是什么？
3. 分析影响：涉及哪些模块，有什么风险？
4. 如果需要 discussion，在 `roadmap/<slug>.md` 中写 Context / Analysis / Decision
5. 结论写入 ROADMAP.md，分配 ID，link detail doc
6. `topo scan .` 更新 graph

### 画图（简单 task）

1. 判断需求属于哪个 section
2. 分配下一个可用数字 ID
3. Link 详情文档（如需要）
4. 写入 ROADMAP.md
5. `topo scan .` 更新 graph

### 执行

```bash
topo update 1.1 status=in-progress    # 开始
topo update 1.1 status=done           # 完成
topo archive                          # 清理
```

## 完整示例

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
