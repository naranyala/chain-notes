# chainnotes documentation

Start here. Each guide answers one kind of question; nothing is duplicated
between them.

| Document | Read it when you want to know… |
| --- | --- |
| [`overview.md`](./overview.md) | What chainnotes is, how it feels to use, and what it deliberately is not |
| [`architecture.md`](./architecture.md) | How the modules fit together — including the command layer and the core/UI boundary — what each one owns, and what happens from launch to first paint |
| [`persistence.md`](./persistence.md) | Where the file lives, the on-disk schema, the debounce and merge rules, and the exact durability guarantee (including what it does *not* promise) |
| [`development.md`](./development.md) | How to build, run, and test; where the workspace file lands on your machine; fixes for common problems |
| [`testing.md`](./testing.md) | The 52 tests, what each suite covers, and the parts of the system that have no automated coverage yet |

For the rules a change must respect and the checked current state, see
[`../PYRAMID-OF-INTENTS.md`](../PYRAMID-OF-INTENTS.md). For what's left to
build, see [`../TODOS.md`](../TODOS.md).
