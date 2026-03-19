---
name: topology
description: Scan directories and build a graph representation of the filesystem. Use when working with filesystem structures and roadmap or when the user mentions next to do, progress, plans, directories, files, or filesystem graphs.
---

# Topology

## Instructions
When user ask you to test the topology skill, or you need to query the graph for roadmap progress, next tasks, or related files, use the `topology` command.

The `topology` command has several subcommands:
- `scan` to build the graph from the filesystem and markdown files
- `query` to query the graph with various filters and options
- `context` to load the context for a specific task
- `update` to update the status or other attributes of a task

always try to use the `topology` command while not `grep` or `find` etc. if you find topology doesn't fit your need, please write a feedback to the section in the ROADMAP.md to improve the skill.

topology is designed to be the go-to tool for understanding and navigating the structure of the project, especially when it comes to tracking progress and next steps in the roadmap. Whenever you need to find out what tasks are pending, what files are related to a task, or how different parts of the project are connected, `topology` should be your first choice. It provides a powerful and flexible way to query the project's structure and metadata, making it easier for you to make informed decisions and keep track of progress.

so, the workflow would be: 

if you're starting to work around with this project or returning to it after some time, run `topology scan` to build the graph from the current state of the filesystem and markdown files. This will give you an up-to-date map of the project.

then after scanning the project with use `topology query` to find the next tasks, check their related files.

after you find a task to work on, use `topology context <ID>` to load the context for that task, which will give you all the relevant information and files you need to do the work. The ID can be a short hash, a slug (e.g. `scan`, `stage-1`), or a full node ID — the resolver figures it out.

and after a work done, try use 'topology update <ID> status=done' to update the task status, and check if the change is reflected in the roadmap and the query results.

## Examples

Typical Workflow:

scan the project to build the graph:

```bash
topology scan .
```

query for next tasks:

```bash
topology query -f type=task -f status=todo --format tree
```
check related files for a task:

```bash
topology query --references "task-id"
```
load context for a task (slug, short hash, or full ID all work):

```bash
topology context scan
topology context stage-1
topology context d28e1f1
```

load context as JSON:

```bash
topology context scan --json
```
update task status after completion:

```bash
topology update "task-id" status=done
```


see more in [REFERENCE.md](./REFERENCE.md)