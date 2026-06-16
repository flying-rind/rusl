# pthread_condattr_setpshared -- Rust 接口归约

## 原始 C 接口
```c
int pthread_condattr_setpshared(pthread_condattr_t *a, int pshared);
```

---

## Rust 外部 ABI 接口

```rust
extern "C" fn pthread_condattr_setpshared(
    a: *mut pthread_condattr_t,
    pshared: core::ffi::c_int,
) -> core::ffi::c_int;
```

[Visibility]: User — 通过 `<pthread.h>` 对外导出 (POSIX)

---

## 意图
设置条件变量是否可以跨进程共享。

## 前置条件
- `a` 为非空指针（`!a.is_null()`），指向有效的 `pthread_condattr_t` 对象
- `pshared` 必须是 `PTHREAD_PROCESS_PRIVATE`（0）或 `PTHREAD_PROCESS_SHARED`（1）

## 后置条件
- Case 1 `pshared == 0`（`PTHREAD_PROCESS_PRIVATE`）：
  - `a.__attr` 最高位清零（非进程共享）
  - 返回 `0`
- Case 2 `pshared == 1`（`PTHREAD_PROCESS_SHARED`）：
  - `a.__attr` 最高位置 1（进程共享）
  - `a.__attr` 低 31 位保持不变
  - 返回 `0`
- Case 3 `pshared > 1`（无效值）：
  - 返回 `EINVAL`
  - `a.__attr` 不变

## 不变量
- `a.__attr` 低 31 位的时钟 ID 在设置进程共享标志时不可丢失

## 算法
```rust
pub extern "C" fn pthread_condattr_setpshared(a: *mut pthread_condattr_t, pshared: c_int) -> c_int {
    if (pshared as c_uint) > 1 {
        return EINVAL;
    }
    unsafe {
        let attr = &mut (*a).__attr;
        *attr = (*attr & 0x7FFF_FFFF) | ((pshared as c_uint) << 31);  // 清除最高位，设置
    }
    0
}
```

## Rust 内部设计要点
- 使用 `pshared as c_uint` 进行无符号比较，消除有符号溢出风险
- 位操作清晰：`& 0x7FFF_FFFF` 清除最高位，`<< 31` 设置新值
- 仅在解引用裸指针时使用 `unsafe`

---

/* Rely */
[RELY]
Predefined Types:
  pthread_condattr_t               // #[repr(C)] 条件变量属性类型
  core::ffi::c_int / c_uint       // Rust 核心库 C FFI 类型

Predefined Constants:
  EINVAL                           // POSIX 错误码（来自 <errno.h>）

[GUARANTEE]
Exported Interface:
  extern "C" fn pthread_condattr_setpshared(a: *mut pthread_condattr_t, pshared: c_int) -> c_int;
                                   // 本模块保证对外提供与 C ABI 兼容的符号
