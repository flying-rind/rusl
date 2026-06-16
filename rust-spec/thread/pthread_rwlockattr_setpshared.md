# pthread_rwlockattr_setpshared -- Rust 接口归约

## 原始 C 接口

```c
int pthread_rwlockattr_setpshared(pthread_rwlockattr_t *a, int pshared);
```

---

## Rust 外部 ABI 接口

```rust
// musl 中该函数无 __ 前缀的主实现，直接以 pthread_rwlockattr_setpshared 导出
pub extern "C" fn pthread_rwlockattr_setpshared(a: *mut pthread_rwlockattr_t, pshared: c_int) -> c_int;
```

---

## 意图

设置读写锁的进程共享属性。控制由该属性对象初始化的读写锁是否可在多个进程之间共享（通过共享内存）。

## 前置条件

- `a` 为非空指针（`!a.is_null()`），指向一个已初始化的有效 `pthread_rwlockattr_t` 对象
- `pshared` 必须为以下值之一:
  - `PTHREAD_PROCESS_PRIVATE` (0): 读写锁仅在同一进程内的线程间共享
  - `PTHREAD_PROCESS_SHARED` (1): 读写锁可在进程间共享

## 后置条件

- Case 1 成功（`pshared` 为 0 或 1）:
  - `(*a).__attr[0] = pshared`
  - 返回 0
- Case 2 失败（`pshared` 无效，即 `pshared > 1`）: 返回 `EINVAL`

## 不变量

无。

## 算法

```
pthread_rwlockattr_setpshared(a, pshared):
  1. if pshared > 1 { return EINVAL }   仅接受 0 或 1
  2. (*a).__attr[0] = pshared           存储进程共享标志
  3. return 0
```

与 `PTHREAD_PROCESS_SHARED` 的兼容性说明：musl 将 `__attr[0]` 作为原始值存储。在 `pthread_rwlock_init` 中，该值乘以 128 后存入 `rw._rw_shared`，用于后续 futex 操作的 `FUTEX_PRIVATE` 标志控制。

---

## Rust 安全包装（模块内部，不对外暴露）

```rust
pub(crate) fn rwlockattr_set_pshared(a: &mut pthread_rwlockattr_t, pshared: c_int) -> c_int {
    if pshared > 1 {
        return EINVAL;
    }
    a.__attr[0] = pshared as c_uint;
    0
}
```

---

/* Rely */
[RELY]
Predefined Structures/Functions:
  (none)                         // 无内部函数依赖，仅做参数校验和赋值

Predefined Constants:
  EINVAL                         // 来自 errno 模块
  PTHREAD_PROCESS_PRIVATE        // 值 = 0
  PTHREAD_PROCESS_SHARED         // 值 = 1

Predefined Types:
  pthread_rwlockattr_t           // 来自 crate 内部类型定义
  c_int, c_uint                  // 来自 core::ffi

[GUARANTEE]
Exported Interface:
  pub extern "C" fn pthread_rwlockattr_setpshared(a: *mut pthread_rwlockattr_t, pshared: c_int) -> c_int;
                                 // 本模块保证对外提供与 C ABI 兼容的 pthread_rwlockattr_setpshared 符号
                                 // 校验 pshared 参数，成功返回 0，无效值返回 EINVAL

Internal Interface:
  pub(crate) fn rwlockattr_set_pshared(a: &mut pthread_rwlockattr_t, pshared: c_int) -> c_int;
                                 // 安全包装，供 crate 内部使用
