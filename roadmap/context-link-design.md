# Context link desigm

根据workflow设计一个好的context cli指令，其实还是要在task id上做设计。

现状：

### Task IDs

Tasks use numeric IDs (e.g. `1.1`, `2.3.1`) as stable identifiers. Use these for all context/update commands:

```bash
topo context 1.3          # by numeric ID (preferred)
topo context scan          # by slug
topo context d28e1f1       # by short hash
```

反馈：
1. numberic id很好生成，查询，但无法联系task文件。

对于
`- [ ] 3.1 Markdown convention spec — [convention](.claude/skills/topology/CONVENTION.md)`
这样的task，结果是这样的：
```
topo context 3.1 
# Markdown convention spec — convention — markdown
task | todo

## Ancestors
  topology
  ROADMAP.md
  Roadmap
  3. Convention & Abstraction
```

2. slug id很难生成，打错一个字母

3. hash基本在这个系统没有什么用处了其实，我觉得是不是可以删去了。

4. 此外，像现在这样的`task content... — `src/query.rs` `这种设计都是很不优雅的。ROADMAP.md task最好就是只有task short description name，具体的背后的context还是放到roadmap/task.md去吧，你觉得呢？

对于`numberic id联系task文件的想法`想法：
对于每个numeric id如有task context文件，则直接在numeric id后引用以numeric id命名task.md地址。

如：
```
- [x] [1.1.1](./roadmap/1-1-1.md) Parse directory structures — `src/scan/directory.rs`
```

优点：agent只要run `topo context 1.1.1` 就找到对应的task描述文件。
缺点：roadmap里会有一堆数字命名的文件，用户难以对应查看文件具体。

理想的使用场景：

用户有一个新feature idea，用户写下来，由agent或用户自己将根据convention转化为Task，并根据需要写具体的task context task.md

由Agent自动执行的时候，Agent会根据这个task loop去找任务和任务相关的context进行执行：

Task loop

Complete cycle for working through tasks:

1. **Find next task**: `topo query -f type=task -f status=todo --format tree`
2. **Pick and mark in-progress**: `topo update <ID> status=in-progress`
3. **Do the work**
4. **Mark done**: `topo update <ID> status=done`
5. **Refresh graph**: `topo scan .`

用户在平行进行添加下一个或者随时添加新task。

同时我们要进行维护roadmap task system和项目本身的alignment。

现在的context返回的result以及context本身的缺陷肯定是不适合agent的。
我们现在就是围绕这个核心workflow应用场景来针对context和task id进行重新设计，找一个更符合agent ergnomic同时又尽量利用文件系统和纯文本格式如markdown的format来兼顾人的可读性的设计平衡。