# cnd_init.c 规约

> musl libc 的 C11 条件变量初始化函数实现。使用复合字面量零初始化条件变量对象。

---

## 依赖图

```
cnd_init
  (无外部函数依赖 — 纯内存零初始化)
```

---

## 函数规约

### 1. cnd_init

```c
int cnd_init(cnd_t *c);
```

[Visibility]: User — 通过 `<threads.h>` 对外导出 (ISO C11 7.26.3.1)

#### Intent

将条件变量 `c` 初始化为全零状态，即为进程内使用的私有条件变量。零初始化等价于 POSIX 默认属性。

#### 前置条件

- `c != NULL`，指向有效的内存位置
- `c` 之前未被初始化（或已被销毁且不再使用）

#### 后置条件

- `*c` 的所有字节设为 0（通过复合字面量 `(cnd_t){ 0 }`）
- 始终返回 `thrd_success` (0)
- 初始化后的条件变量可用于 `cnd_wait`、`cnd_timedwait`、`cnd_signal`、`cnd_broadcast`

#### 系统算法

```
cnd_init(c):
  1. *c = (cnd_t){ 0 }   // 零初始化
  2. return thrd_success
```

#### 不变量

- 零初始化产生默认属性（私有、进程内条件变量）

#### 依赖

- `cnd_t` — 等同于 `pthread_cond_t` 的 typedef，实际为包含 12 个 `int` 的 union struct（见 `<bits/alltypes.h>`）
- `thrd_success` — 枚举值 `0`
