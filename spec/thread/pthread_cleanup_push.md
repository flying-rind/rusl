# pthread_cleanup_push.c 规约

> musl libc 的线程取消清理处理函数（push/pop）实现。提供 `_pthread_cleanup_push` 和 `_pthread_cleanup_pop` 底层函数，由 `<pthread.h>` 中的宏 `pthread_cleanup_push` / `pthread_cleanup_pop` 调用。

---

## 依赖图

```
pthread_cleanup_push (宏 -> _pthread_cleanup_push)
  └─> __do_cleanup_push(cb)    — hidden (见 pthread 内部实现)

pthread_cleanup_pop (宏 -> _pthread_cleanup_pop)
  └─> __do_cleanup_pop(cb)     — hidden (见 pthread 内部实现)
```

---

## 内部函数规约

### 1. dummy (static)

```c
static void dummy(struct __ptcb *cb);
```

[Visibility]: Internal (不导出) — static 占位空函数，分别在启动早期充当 `__do_cleanup_push` 和 `__do_cleanup_pop` 的弱别名占位

#### Intent

在线程初始化完成前，清理 push/pop 操作是无操作。一旦真实实现被链接，弱别名被覆盖。

---

## 对外导出函数规约

### 2. _pthread_cleanup_push

```c
void _pthread_cleanup_push(struct __ptcb *cb, void (*f)(void *), void *x);
```

[Visibility]: User — 通过 `<pthread.h>` 声明，被宏 `pthread_cleanup_push` 使用

#### Intent

将清理处理函数 `f` 及其参数 `x` 压入调用线程的取消清理栈。若当前线程在 `pthread_cleanup_pop` 之前被取消，`f(x)` 将被自动调用。

#### 前置条件

- `cb != NULL`，为栈分配的 `struct __ptcb` 对象（通常由宏在栈上创建）
- `f != NULL`，为清理回调函数
- 必须与 `_pthread_cleanup_pop` 配对使用，且在同一词法作用域内

#### 后置条件

- `cb->__f = f`，`cb->__x = x`（保存清理回调及参数）
- `__do_cleanup_push(cb)` 将 `cb` 链接到当前线程的取消清理栈（`self->cancelbuf` 链表头）
- 若线程在执行 `_pthread_cleanup_pop` 之前被取消，清理栈从 top 向下依次调用各 `__f(__x)`

#### 系统算法

```
_pthread_cleanup_push(cb, f, x):
  1. cb->__f = f
  2. cb->__x = x
  3. __do_cleanup_push(cb)  // 压入线程本地的清理栈
```

---

### 3. _pthread_cleanup_pop

```c
void _pthread_cleanup_pop(struct __ptcb *cb, int run);
```

[Visibility]: User — 通过 `<pthread.h>` 声明，被宏 `pthread_cleanup_pop` 使用

#### Intent

从调用线程的取消清理栈中弹出顶部处理函数。若 `run != 0`，则在弹出前执行清理函数。

#### 前置条件

- `cb` 必须与先前的 `_pthread_cleanup_push` 配对，指向同一个 `struct __ptcb`
- `cb` 必须位于当前清理栈的顶部

#### 后置条件

- `__do_cleanup_pop(cb)` 将 `cb` 从当前线程的清理栈中移除
- 若 `run != 0`：执行 `cb->__f(cb->__x)`（调用清理回调）
- 若 `run == 0`：清理回调不被执行

#### 系统算法

```
_pthread_cleanup_pop(cb, run):
  1. __do_cleanup_pop(cb)    // 从清理栈中弹出
  2. if run: cb->__f(cb->__x) // 若要求, 执行清理函数
```

---

## 宏展开说明

`<pthread.h>` 中的宏定义：

```c
#define pthread_cleanup_push(f, x) do { struct __ptcb __cb; _pthread_cleanup_push(&__cb, f, x);
#define pthread_cleanup_pop(r) _pthread_cleanup_pop(&__cb, (r)); } while(0)
```

它们在用户代码中必须成对出现在同一词法作用域内：

```c
pthread_cleanup_push(cleanup_handler, arg);
// ... 临界区代码 ...
pthread_cleanup_pop(execute_flag);  // execute_flag != 0 时执行清理
```

#### 不变量

- 每个 `pthread_cleanup_push` 必须对应一个 `pthread_cleanup_pop`，且在同一作用域内
- 清理栈符合 LIFO（后进先出）顺序

#### 依赖

- `struct __ptcb` — 线程取消清理控制块（定义于 `<pthread.h>`）：
  ```c
  struct __ptcb {
      void (*__f)(void *);   // 清理回调函数指针
      void *__x;             // 回调参数
      struct __ptcb *__next; // 链表下一节点
  };
  ```
- `__do_cleanup_push(cb)` / `__do_cleanup_pop(cb)` — 操作当前线程 `cancelbuf` 链表（hidden，在 pthread 模块其他文件中实现）
- `self->cancelbuf` — 当前线程 `struct pthread` 中的清理栈链表头（定义于 `internal/pthread_impl.h`）
