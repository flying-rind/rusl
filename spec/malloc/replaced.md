# replaced.c 规约

> **文件路径**: `src/malloc/replaced.c`
> **复杂度层级**: Level 1 (仅需前置/后置条件)
> **模块类别**: 内部全局状态定义 — 无函数实现，仅定义全局变量

---

## 依赖图

```
replaced.c (无函数调用)
  └─ 被以下模块读取:
       ├─ ldso/dynlink.c (写入: __malloc_replaced, __aligned_alloc_replaced)
       ├─ src/malloc/calloc.c (读取: __malloc_replaced)
       ├─ src/malloc/oldmalloc/aligned_alloc.c (读取: __malloc_replaced, __aligned_alloc_replaced)
       └─ src/malloc/mallocng/glue.h (读取: 两者, 通过宏 DISABLE_ALIGNED_ALLOC)
```

---

## 模块概述

本文件是 musl libc 中**最简洁的源文件之一**——它仅定义两个全局 `int` 变量，不包含任何函数。这两个变量充当 **malloc 系列函数插替检测标志 (interposition detection flags)**，是 musl 应对 ELF 符号插替 (symbol interposition) 机制的核心基础设施。

在 ELF 动态链接模型中，应用程序或预加载的共享库可以通过定义同名符号来"插替"（override）libc 提供的 `malloc` 等函数。当这种情况发生时，musl 内部的 `calloc`、`aligned_alloc` 等函数如果继续调用内部 `malloc` 实现，将产生不一致行为（例如：用户替换了 `malloc` 但未替换 `calloc`，而 musl 的 `calloc` 内部可能直接操作内部 `malloc` 返回的内存块元数据）。这套标志机制允许 musl 在运行时检测插替状态并切换到安全路径。

---

## 全局变量

### `__malloc_replaced`

```c
int __malloc_replaced;
```

[Visibility]: Internal — musl 内部状态变量，POSIX/C 标准未定义。声明于 `src/internal/dynlink.h`，`hidden` 可见性，不对用户程序暴露。

#### 语义

指示标准 `malloc` 函数是否已被外部代码插替 (interposed)。

| 值 | 含义 |
|----|------|
| `0` | `malloc` **未被**替换 — musl 内部实现为唯一提供者。`calloc` 可使用内部优化（如 `__malloc_allzerop` 快速清零检查），动态链接器可安全使用内部 `realloc`。 |
| `1` (非零) | `malloc` **已被**替换 — 外部实现覆盖了 musl 的 `malloc`。musl 必须切换到"防御性"模式：禁用依赖内部 malloc 元数据的优化，在特定路径中避免使用 `realloc`。 |

#### 生命周期与状态转换

```
初始值: 0 (程序启动时, BSS 段零初始化)
  │
  │  动态链接器加载所有共享库后执行符号查找:
  │  if (find_sym(head, "malloc", 1).dso != &ldso)
  │       __malloc_replaced = 1;
  │
  ▼
最终值: 0 或 1 (在动态链接器完成加载后确定，此后只读)
```

**不变量**: 一旦动态链接器完成所有共享库的加载和重定位（进入 `runtime = 1` 模式），`__malloc_replaced` 的值不再改变。任何后续代码仅读取此值。

#### 读取者 (消费方)

| 位置 | 影响 |
|------|------|
| `src/malloc/calloc.c:41` | 若 `!__malloc_replaced`，使用 `__malloc_allzerop(p)` 快速判断内存是否全零；若已替换，跳过此优化以保持兼容。 |
| `ldso/dynlink.c:1311` | 若 `__malloc_replaced && !p->runtime_loaded`，动态链接器不使用 `realloc` 扩展依赖数组，改为手动分配新数组并复制，防止调用被替换的 `realloc`。 |
| `ldso/dynlink.c:1620` | 若 `!__malloc_replaced` 且主构造函数队列非内置，则调用 `free` 释放临时队列（信任内部实现）。 |
| `src/malloc/mallocng/glue.h:42` | 与 `__aligned_alloc_replaced` 联合用于 `DISABLE_ALIGNED_ALLOC` 宏。 |

---

### `__aligned_alloc_replaced`

```c
int __aligned_alloc_replaced;
```

[Visibility]: Internal — musl 内部状态变量，POSIX/C 标准未定义。声明于 `src/internal/dynlink.h`，`hidden` 可见性，不对用户程序暴露。

#### 语义

指示标准 `aligned_alloc` 函数是否已被外部代码插替。

| 值 | 含义 |
|----|------|
| `0` | `aligned_alloc` **未被**替换 — musl 内部实现为唯一提供者。 |
| `1` (非零) | `aligned_alloc` **已被**替换 — 外部实现覆盖了 musl 的版本。 |

#### 生命周期与状态转换

```
初始值: 0 (程序启动时, BSS 段零初始化)
  │
  │  动态链接器加载所有共享库后执行符号查找:
  │  if (find_sym(head, "aligned_alloc", 1).dso != &ldso)
  │       __aligned_alloc_replaced = 1;
  │
  ▼
最终值: 0 或 1 (在动态链接器完成加载后确定，此后只读)
```

**不变量**: 与 `__malloc_replaced` 相同，在动态链接器进入运行时模式后不可变。

#### 读取者 (消费方)

| 位置 | 影响 |
|------|------|
| `src/malloc/oldmalloc/aligned_alloc.c:16` | 若 `__malloc_replaced && !__aligned_alloc_replaced`，`aligned_alloc` 返回 `ENOMEM` 并设置 `errno`，因为在此场景下无法安全实现对齐分配。 |
| `src/malloc/mallocng/glue.h:42` | 定义 `DISABLE_ALIGNED_ALLOC` 宏 = `(__malloc_replaced && !__aligned_alloc_replaced)`，控制新版 malloc 实现中对齐分配功能的启用/禁用。 |

---

## 系统不变量 (System Invariants)

1. **写入单调性**: 这两个变量由 BSS 零初始化后，仅在动态链接器初始化阶段（`ldso/dynlink.c` 的 `__dls3` 函数末尾，进入 `runtime = 1` 之前）被写入 **最多一次**。写入后永不回退为 0。

2. **读取线程安全性**: 变量仅被写入一次（在单线程启动阶段），之后所有访问均为只读，因此无需显式同步机制即可保证多线程安全。

3. **写入者唯一性**: 只有 `ldso/dynlink.c` 中的动态链接器代码负责写入这两个变量。musl libc 中其他所有代码均为只读消费者。

4. **部分替换兼容性**: musl 通过两个独立标志处理"部分替换"场景：
   - `malloc` 被替换但 `aligned_alloc` 未被替换 (`__malloc_replaced=1, __aligned_alloc_replaced=0`)：对齐分配功能被禁用，因为 musl 的 `aligned_alloc` 依赖内部 `malloc` 实现细节。
   - 两者均被替换 (`__malloc_replaced=1, __aligned_alloc_replaced=1`)：对齐分配委托给替换实现，`calloc` 跳过内部优化。
   - 仅 `aligned_alloc` 被替换而 `malloc` 未替换 (`__malloc_replaced=0, __aligned_alloc_replaced=1`)：理论上可能但实际极少发生；此时内部 `aligned_alloc` 仍正常工作，但替换实现不会收到调用（因为 musl 内部可能直接调用而非通过 PLT）。

---

## 设计意图 (Intent)

`replaced.c` 体现了 musl 对 **ELF 符号插替兼容性** 的精心设计。标准 C 库规范允许用户替换 `malloc` 系列函数，但替换不需要覆盖全部变体（如仅替换 `malloc`/`free` 而不替换 `calloc`）。若 musl 的 `calloc` 在内部直接操作 `malloc` 返回的 chunk 元数据，而用户替换的 `malloc` 使用了不兼容的内部布局，则会导致内存损坏。

本模块通过两个全局标志将"是否有外部插替"的信息从动态链接器传递到 malloc 子系统，使 `calloc`、`aligned_alloc` 等函数在检测到插替时自动降级为安全路径（放弃依赖内部元数据的优化），从而在不牺牲默认性能的前提下保证替换兼容性。