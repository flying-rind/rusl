# pthread_self.c 规约

> musl libc 的线程自身标识符获取函数实现。通过 TLS（线程局部存储）寄存器直接获取当前线程的 `pthread_t`，无系统调用。

---

## 依赖图

```
pthread_self (弱别名 -> __pthread_self_internal)
  └─> __pthread_self()          — see internal/pthread_impl.h (宏, 展开为 __get_tp())
thrd_current (C11, 弱别名 -> __pthread_self_internal)
  └─> __pthread_self()          — 同上
```

---

## 内部静态函数规约

### 1. __pthread_self_internal (static)

```c
static pthread_t __pthread_self_internal(void);
```

[Visibility]: Internal (不导出) — static 函数，内联转发到 `__pthread_self()`

#### Intent

封装对 `__pthread_self()` 的调用，为 `pthread_self`（POSIX）和 `thrd_current`（C11）提供统一的弱别名目标。

#### 系统算法

```
__pthread_self_internal():
  1. return __pthread_self()
```

---

## 对外导出函数规约

### 2. pthread_self

```c
pthread_t pthread_self(void);
```

[Visibility]: User — 通过 `<pthread.h>` 对外导出 (POSIX)，`__attribute__((const))` 声明为纯函数（给定参数总是返回相同值，无副作用）

#### Intent

返回调用线程的线程标识符（`pthread_t`）。无需系统调用，直接通过 TLS 寄存器读取。

#### 前置条件

- 调用线程必须已通过 `pthread_create` 创建，或为主线程（主线程的 `pthread_t` 由 libc 初始化时设置）

#### 后置条件

- 返回值为调用线程的 `pthread_t`，即指向当前线程 `struct __pthread` 的指针（musl 中 `pthread_t` 是指针类型）
- 无副作用，不修改任何全局或线程局部状态
- 保证返回值在调用线程的生命周期内有效且唯一

#### 系统算法

```
pthread_self():
  1. return __pthread_self_internal()
     = __pthread_self()
     = (pthread_t)__get_tp()   // 读取 TLS 线程指针寄存器
```

### 3. thrd_current

```c
pthread_t thrd_current(void);
```

[Visibility]: User — 通过 `<threads.h>` 对外导出 (ISO C11)，是 `__pthread_self_internal` 的弱别名

#### Intent

C11 标准版的"获取当前线程标识符"，等同于 `pthread_self()`。

#### 系统算法

```
thrd_current():
  1. return __pthread_self_internal()
```

---

#### 不变量

- 每个线程的 `pthread_t` 在其生命期内不变且唯一

#### 依赖

- `__pthread_self()` — 宏定义于 `internal/pthread_impl.h`，展开为 `(pthread_t)__get_tp()` 或 `(pthread_t)(__get_tp() - sizeof(struct __pthread) - TP_OFFSET)`，取决于 TLS_ABOVE_TP 配置
- `__get_tp()` — 架构特定的内联汇编或内置函数，读取 TLS 线程指针寄存器（如 x86-64 的 `fs` 段寄存器）
