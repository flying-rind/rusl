# __syscall_cp.c 规约

> musl libc 内部带取消点检查的系统调用包装器。在调用底层 `__syscall` 之前检查线程取消点，使得可被取消的系统调用（如 `read`、`write`、`futex wait` 等）在取消请求挂起时能正确响应。

---

## 依赖图

```
sccp (static) → __syscall_cp_c (weak alias)
  └─> __syscall(nr, u, v, w, x, y, z)     (see syscall.h — 原始系统调用)

__syscall_cp (导出)
  └─> __syscall_cp_c(nr, u, v, w, x, y, z)  (see 本文件 — 弱别名指向 sccp)
```

---

## 函数规约

### 1. sccp (static)

```c
static long sccp(syscall_arg_t nr,
                 syscall_arg_t u, syscall_arg_t v, syscall_arg_t w,
                 syscall_arg_t x, syscall_arg_t y, syscall_arg_t z);
```

[Visibility]: Internal (不导出) — 文件作用域静态函数

#### Intent

通过弱别名 `weak_alias(sccp, __syscall_cp_c)` 作为 `__syscall_cp_c` 的默认实现。在单线程/无取消支持的配置中，直接调用 `__syscall` 执行系统调用。多线程时由 `__syscall_cp_c` 的实际实现（在架构相关的汇编文件中）覆盖，该实现会在调用 `__syscall` 前检查取消点。

#### 前置条件

- `nr` 为有效的系统调用号
- 其余参数为系统调用参数（类型 `syscall_arg_t`，通常为 `long`）

#### 后置条件

- 直接委托给 `__syscall(nr, u, v, w, x, y, z)`
- 返回值为系统调用结果（原始返回值，含负的 errno）

#### 系统算法

```
sccp(nr, u, v, w, x, y, z):
  1. return __syscall(nr, u, v, w, x, y, z)
```

#### 依赖

- `__syscall()` — 原始系统调用宏（见 `syscall.h`）

---

### 2. __syscall_cp (导出符号)

```c
long (__syscall_cp)(syscall_arg_t nr,
                    syscall_arg_t u, syscall_arg_t v, syscall_arg_t w,
                    syscall_arg_t x, syscall_arg_t y, syscall_arg_t z);
```

[Visibility]: Internal (不导出) — 被 `syscall.h` 声明为 hidden，仅 musl 内部使用。注意函数名带有括号 `(__syscall_cp)` 以防止与同名宏展开冲突。

#### Intent

带取消点检查的系统调用入口。函数定义本身直接委托给 `__syscall_cp_c`。实际的取消点检查逻辑在架构相关的汇编实现 `__syscall_cp_c` 中（该汇编实现在调用系统调用前检查 `cancel` 标志，并在需要时调用取消处理）。在单线程构建中，`__syscall_cp_c` 通过弱别名回退到 `sccp`（即直接调用 `__syscall`）。

#### 前置条件

- `nr` 为有效的系统调用号
- 参数与目标系统调用的 ABI 要求匹配

#### 后置条件

- 若线程有挂起的取消请求，在系统调用前执行取消处理（由 `__syscall_cp_c` 的汇编实现处理）
- 返回值为系统调用结果

#### 系统算法

```
(__syscall_cp)(nr, u, v, w, x, y, z):
  1. return __syscall_cp_c(nr, u, v, w, x, y, z)
```

#### 不变量

- `__syscall_cp_c` 的实现保证：若取消请求挂起且取消未被禁用，系统调用不会执行
- 函数名用括号包围是为了避免预处理器将 `__syscall_cp(...)` 调用递归展开

#### 依赖

- `__syscall_cp_c()` — 实际实现（外部：架构相关汇编)
- `syscall_arg_t` — 系统调用参数类型（见 `syscall.h`）
