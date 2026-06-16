# clone.c 规约

> musl libc 内部 `clone` 系统调用的占位实现。在 musl 中，`clone` 的实际实现位于架构相关的汇编文件中（如 `x86_64/clone.s`），此 C 文件提供返回 `-ENOSYS` 的默认回退实现。

---

## 依赖图

```
__clone → 返回 -ENOSYS (无实际依赖)
```

---

## 函数规约

### 1. __clone

```c
int __clone(int (*func)(void *), void *stack, int flags, void *arg, ...);
```

[Visibility]: Internal (不导出) — 被 `pthread_impl.h` 声明为 hidden，仅 musl 内部使用。实际实现由架构相关的汇编文件提供。

#### Intent

此 C 源文件是 `clone` 系统调用的占位实现。在正常情况下，此文件被架构相关的汇编实现（`arch/<arch>/clone.s`）替代，后者通过 `SYS_clone` 系统调用创建新线程。

此 C 版本的意图是提供一个安全的回退：在未提供汇编实现的架构上，返回 `-ENOSYS` 表示不支持。

#### 前置条件

- 无（在接受任何参数前即返回 `-ENOSYS`）

#### 后置条件

- 始终返回 `-ENOSYS`（功能未实现）

#### 系统算法

```
__clone(func, stack, flags, arg, ...):
  1. return -ENOSYS
```

#### 不变量

- 此函数永不创建线程——始终返回 `-ENOSYS`
- 架构相关的汇编实现（如 `arch/x86_64/clone.s`）在链接时替代此符号

#### 依赖

- `errno.h` — 提供 `ENOSYS` 错误码
- `pthread_impl.h` — 函数声明及类型定义
