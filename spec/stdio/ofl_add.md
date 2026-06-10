# ofl_add.c 规约

> musl libc `__ofl_add` 实现 — 将新打开的 FILE 对象添加到全局打开文件链表。通常在 `fopen` / `fdopen` 等函数内部调用。

---

## 依赖图

```
__ofl_add (Internal)
  ├── __ofl_lock (see ofl.c spec)
  └── __ofl_unlock (see ofl.c spec)
```

---

## 函数规约

### 1. \_\_ofl_add

```c
FILE *__ofl_add(FILE *f);
```

[Visibility]: Internal — `hidden` 可见性，musl 内部使用

#### Intent

将新打开的 FILE 对象 `f` 插入全局打开文件链表的头部。该双向链表在程序退出时由 `__stdio_exit` 遍历，以刷新所有缓冲数据。返回值 `f` 允许调用方链式使用（如 `return __ofl_add(f)`）。

#### 前置条件

- `f`: 有效的 `FILE *` 指针，指向新打开的流
- `f` 尚未在全局链表中（即 `f->next` 和 `f->prev` 尚未有意义）
- 全局链表锁 `ofl_lock` 未被当前线程持有

#### 后置条件

- `f` 被插入全局链表头部
- 链表指针调整：
  - `f->next` 指向插入前链表的头部（可能为 NULL）
  - 若原头部非空，原头部的 `prev` 指向 `f`
  - `f->prev` 为 NULL（头部节点）
  - 全局链表头 `ofl_head` 指向 `f`
- 链表锁被释放
- 返回 `f`（与输入相同）

#### 系统算法

```
__ofl_add(f):
  1. head = __ofl_lock() — 获取锁并取得链表头指针的地址
  2. f->next = *head — 新节点后继指向原链表头
  3. 若 *head != NULL:
       (*head)->prev = f — 原头的 prev 指向新节点
  4. *head = f — 更新链表头为新节点
  5. __ofl_unlock() — 释放锁
  6. 返回 f
```

#### Intent 关键设计点

- **头部插入策略**：始终将新流添加到链表头部，O(1) 时间，无需遍历
- **双向链表维护**：正确处理 `prev` 和 `next` 指针，确保 `prev` 指针可用于反向遍历（如 `fclose` 中的链表移除）
- **原子性**：链表修改在锁保护下完成，保证线程安全

#### 不变量

- 全局打开文件链表始终是双向链表：若 `A->next == B`，则 `B->prev == A`
- 头部节点的 `prev` 始终为 NULL
- 尾部节点的 `next` 始终为 NULL
- 任何对链表的修改必须在持有 `ofl_lock` 的前提下进行

#### 依赖

- `__ofl_lock()` — 获取全局文件链表锁并返回头指针（定义于 `src/stdio/ofl.c`）
- `__ofl_unlock()` — 释放全局文件链表锁（定义于 `src/stdio/ofl.c`）
