# call_once.c 规约

> musl libc 的 C11 一次性执行函数实现。是 POSIX `__pthread_once` 的直接转发包装。

---

## 依赖图

```
call_once
  └─> __pthread_once(flag, func)  — see pthread_impl.h (内部实现)
```

---

## 函数规约

### 1. call_once

```c
void call_once(once_flag *flag, void (*func)(void));
```

[Visibility]: User — 通过 `<threads.h>` 对外导出 (ISO C11 7.26.2.1)

#### Intent

确保 `func` 在由同一 `once_flag` 控制的多次 `call_once` 调用中恰好执行一次。是 `__pthread_once` 的纯转发包装器。

#### 前置条件

- `flag != NULL`，指向通过 `ONCE_FLAG_INIT` 初始化的 `once_flag` 对象
- `func != NULL`，指向无参数、无返回值的有效函数
- `flag` 的生命周期覆盖所有使用它的 `call_once` 调用

#### 后置条件

- Case 1 `func` 尚未执行过：`func` 被调用一次，之后 `flag` 标记为"已完成"。所有并发或后续 `call_once` 调用将阻塞等待完成或立即返回。
- `func` 在所有线程中恰好执行一次（即使并发调用）
- 若 `func` 未执行完时另有线程调用 `call_once`：该线程阻塞等待直到 `func` 执行完毕
- 函数无返回值

#### 系统算法

```
call_once(flag, func):
  1. __pthread_once(flag, func)   // 委托 POSIX 内部实现
  2. return
```

#### 不变量

- 同一 `once_flag` 全局最多仅执行一次 `func`

#### 依赖

- `__pthread_once()` — POSIX `pthread_once` 的内部实现（见 `pthread_impl.h`）
- `once_flag` — `typedef int`，值为 `0` (未执行) 或非零 (已完成)
- `ONCE_FLAG_INIT` — 宏，定义为 `0`
