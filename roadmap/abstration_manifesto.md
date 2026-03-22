# Abstration
 
Goal: 将topology的定位升级为为agent设计的task driven “自然语言 → topo pipeline” 的最小 prompt / 转换规则 cli。

Topology 不仅仅是一个 CLI，而是一个 “可被编程的结构提取语言（meta-CLI）”

agents通过读取topology skills和使用topo cli，可以把“结构理解”变成一种可以被编程、组合、复用的操作语言。

如何设计一个足够简单，但足够通用的“结构提取原语（primitives）”集合，让 agent 可以组合出任意 parser？

不定义统一 schema,不定义统一 graph

而是：定义了一组结构操作指令

topo并非要减少 agent 需要阅读的 raw text，而是配合ROADMAP.md和roadmap/task.md提供一个可持续更新人机协作工作流。
topo cli作用不是只让agent只通过cli得到结构就理解任务具体如何执行，它更像是一个将一个project转化为一个地图，topo cli是绘图和解析地图的工具，topo的使用实际像是不断给project画更全面的地图，这样每次agent可以在阅读下一个task(task本身也是一个地图/roadmap)后根据地图找目的地，而非每次都重复盲目阅读project文件，具体context还是要agent去阅读的。

同时还需要注意的是，topo的作用虽然是一个用于提取结构的meta cli，但是它是为agent服务，根源其实是为人服务的，也就是它是其实以human task为导向为agent所使用设计的。偏“意图导向”，而不是路径导向。

## New Agent Human Interface 人机协作Workflow

- 对于用户给出的在文本框用自然语言描述的比较大的Goal，agent会在理解后将其翻译为topo pipeline，并写入更新到ROADMAP.md的task list。

- 在Goal确立后，对于渐进式的项目推进，Agent会调用直接使用先前已经准备好的topo script，用于在不需要逐一阅读大量raw text context而直接从抽象structure维度来确定下一步要做得事以及注意事项。

Human:
想法（自然语言）/ Roadmap task list

Agent:
编译成 topo pipeline, 这一步，agent做的是：

1. 先对于任务进行非结构化，text形式的思考：

```
e.g.
This project seems to be a fullstack app.

I expect:
- a backend in Python
- a frontend in React
- some form of API or coupling
- maybe a database

To verify this, I need to inspect:
- filesystem structure
- markdown docs (roadmap)
- possible database schema
```

2. 感知计划（半结构化）

Plan:
- inspect filesystem
- inspect markdown: roadmap.md
- (optional) inspect database

3. 在meta topo的基础上编译用于针对现有的项目文件结构abstract规则，这些规则会被保存下来。
  3.1. 如果项目是空白，则自己编写template，之后的文件和结构更新也都follow这个准则。
  3.2. 如果项目已有内容，则在现在的基础上编写。

4. agent直接运行topo scan进行实际的topo结构提取。
  4.1. 如果有错误，在3.4间反复修改。但最多不超过3次。超过3次停止回复用户告诉出错原因。

5. 之后对于渐进式的task执行，agent则直接使用包装好topo scan，如若有新结构引入，则重复3至4步。

Topo:
执行结构提取

Agent:
根据topo的结构指引去完成任务。

对于第一次bootstrap盲扫既可以是有default最低起点，也是允许让agent运行如grep, ls，或直接阅读README.md, ROADMAP.md等指令来确定pipeline的。

最终会变为：

    (1) task + context
            ↓
    (2) agent 形成“模糊结构理解”（自然语言）
            ↓
    (3) agent 决定：需要观察哪些 object（filesystem / md / db）
            ↓
    (4) 翻译成 topo script（可执行）
            ↓
    (5) topo scan（获取现实信息）
            ↓
    (6) agent 用文本方式做 diff / 反思
            ↓
    (7) 更新理解
            ↓
    (8) 执行
    (loop)

## 疑问

1. 这个meta topo cli要怎么设计才能像python里的function abstract一样成为结构提取抽象化元cli
2. 缺陷和潜在的问题
