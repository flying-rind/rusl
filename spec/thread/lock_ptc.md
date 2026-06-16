# lock_ptc.c 规约

> musl libc 内部 pthread cancel (PTC) 锁包装器。使用全局读写锁保护线程取消操作的关键区间，防止在多线程环境中并发的线程创建/取消引发竞争条件。

---

## 依赖图

```
__inhibit_ptc
  └─> pthread_rwlock_wrlock(&lock)  (see pthread_rwlock_wrlock — POSIX 读写锁写锁定)

__acquire_ptc
  └─> pthread_rwlock_rdlock(&lock)  (see pthread_rwlock_rdlock — POSIX 读写锁读锁定)

__release_ptc
  └─> pthread_rwlock_unlock(&lock)  (see pthread_rwlock_unlock — POSIX 读写锁解锁)
```

---

## 全局变量

### lock

```c
static pthread_rwlock_t lock = PTHREAD_RWLOCK_INITIALIZER;
```

[Visibility]: Internal (不导出) — 文件作用域静态变量

#### Intent

全局读写锁，用于协调 PTC（pthread cancel）相关操作。初始化为静态初始化值 `PTHREAD_RWLOCK_INITIALIZER`，在程序启动时即可使用。

#### 不变量

- `lock` 始终处于有效状态（已初始化、未销毁）
- 任意时刻至多一个线程持有 `lock` 的写锁
- 多个线程可以同时持有 `lock` 的读锁

---

## 函数规约

### 1. __inhibit_ptc

```c
void __inhibit_ptc(void);
```

[Visibility]: Internal (不导出) — 被 `pthread_impl.h` 声明为 hidden，仅 musl 内部使用

#### Intent

以写模式获取全局 PTC 读写锁，排他性地禁止线程取消操作。通常在需要原子地创建/销毁线程或进行其他不可被取消中断的操作前调用。写锁确保在关键区间内没有并发线程可以读取 PTC 状态。

#### 前置条件

- 全局 PTC 锁 `lock` 已初始化
- 调用者不持有该锁（无论是读锁还是写锁）

#### 后置条件

- 调用者持有 `lock` 的写锁
- 所有其他线程对 `__acquire_ptc` / `__release_ptc` 的调用均被阻塞
- 调用者处于"PTC 被禁止"状态

#### 依赖

- `pthread_rwlock_wrlock()` — POSIX 读写锁写锁定（外部：libc pthread 实现）
- `PTHREAD_RWLOCK_INITIALIZER` — POSIX 读写锁静态初始化宏

---

### 2. __acquire_ptc

```c
void __acquire_ptc(void);
```

[Visibility]: Internal (不导出) — 被 `pthread_impl.h` 声明为 hidden，仅 musl 内部使用

#### Intent

以读模式获取全局 PTC 读写锁。允许多个线程同时持有读锁，但若有线程持有写锁则阻塞。通常在可能被取消的代码路径中调用，以读取 PTC 状态。

#### 前置条件

- 全局 PTC 锁 `lock` 已初始化
- 调用者不持有该锁的写锁（可持有读锁，视实现而定）

#### 后置条件

- 调用者持有 `lock` 的读锁
- 若有线程持有写锁，则阻塞直到写锁被释放

#### 依赖

- `pthread_rwlock_rdlock()` — POSIX 读写锁读锁定（外部：libc pthread 实现）

---

### 3. __release_ptc

```c
void __release_ptc(void);
```

[Visibility]: Internal (不导出) — 被 `pthread_impl.h` 声明为 hidden，仅 musl 内部使用

#### Intent

释放由 `__inhibit_ptc` 或 `__acquire_ptc` 获取的全局 PTC 读写锁（无论读锁还是写锁）。

#### 前置条件

- 全局 PTC 锁 `lock` 已初始化
- 调用者持有该锁（读锁或写锁）

#### 后置条件

- 调用者释放持有的锁
- 若该锁是写锁且有待处理的读/写请求，其中一个被唤醒
- 若该锁是读锁且是最后一个读锁，待处理的写请求可能被唤醒

#### 依赖

- `pthread_rwlock_unlock()` — POSIX 读写锁解锁（外部：libc pthread 实现）
