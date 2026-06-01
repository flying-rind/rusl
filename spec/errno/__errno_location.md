# \_\_errno_location.c 规约

> musl libc 内部 `errno` 线程局部存储访问器实现。返回当前线程的 `errno` 变量地址。

---

## 依赖图

```
__errno_location
  └─> __pthread_self  (see pthread_self.c spec)
```

---

## 函数规约

### 1. \_\_errno_location

```c
int *__errno_location(void);
```

[Visibility]: Internal — musl 内部实现，不直接对外暴露。通过宏 `errno`（定义于 `<errno.h>`）间接由用户代码使用

#### Intent

返回当前线程的 `errno` 变量地址。由于 musl 是多线程安全的，每个线程拥有独立的 `errno` 存储（位于线程控制块 `pthread` 结构体中）。用户代码通过 `errno` 宏访问该地址。

#### 前置条件

- 调用线程已初始化（线程指针已设置）
- `__pthread_self()` 返回有效的线程控制块指针

#### 后置条件

- 返回指向当前线程 `errno_val` 字段的 `int*` 指针
- 不设置 errno
- 线程安全：不同线程返回不同的地址

#### 系统算法

```
__errno_location():
  return &__pthread_self()->errno_val
```

#### 不变量

- 对于给定的线程 `T`，始终返回相同的地址（`errno_val` 位于线程控制块内）
- 不同线程的返回值互不相同

#### 依赖

- `__pthread_self()` — 获取当前线程控制块指针（定义于 `src/thread/pthread_self.c`）

---

### 2. \_\_\_errno_location (weak_alias)

```c
weak_alias(__errno_location, ___errno_location);
```

[Visibility]: Internal — 三下划线别名，供某些需要直接符号引用的场景使用

- **Intention**: 提供额外的弱别名 `___errno_location`，使 `errno` 宏可通过不同符号名访问同一实现。

前置/后置条件及行为：完全等同于 `__errno_location`。
