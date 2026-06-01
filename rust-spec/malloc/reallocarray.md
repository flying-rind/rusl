# reallocarray — Rust 接口归约

## 原始 C 接口

```c
void *reallocarray(void *ptr, size_t m, size_t n);
```

---

## Rust 外部 ABI 接口

```rust
#[no_mangle]
pub unsafe extern "C" fn reallocarray(ptr: *mut core::ffi::c_void, m: usize, n: usize) -> *mut core::ffi::c_void;
```

[Visibility]: External -- BSD 扩展函数，`<stdlib.h>` 声明。通过 `extern "C"` 和 `#[no_mangle]` 保证 ABI 兼容性，外部 C 代码可透明调用。

**ABI 兼容性保证**:
- 使用 `extern "C"` 调用约定
- 参数 `ptr: *mut c_void` 对应 C 的 `void *ptr`
- 参数 `m: usize` 对应 C 的 `size_t m`
- 参数 `n: usize` 对应 C 的 `size_t n`
- 返回值 `*mut c_void` 对应 C 的 `void *`
- 使用 `#[no_mangle]` 确保符号名为 `reallocarray`
- 不得改变参数顺序、类型宽度、返回值语义，必须满足所有 C spec 规约约束

---

## 意图

对 `realloc(ptr, m * n)` 做安全的整数溢出检查版本。分配 `m * n` 个字节的内存（`m` 个元素，每个元素 `n` 字节），并在 `m * n` 乘法溢出 `usize` 时返回 `null` 并设置 `errno = ENOMEM`，而不是产生一个错误且可能极小的分配结果。该函数是 OpenBSD 首创的 BSD 扩展，用于防御整数溢出漏洞。

## 前置条件

- 若 `ptr` 非 null，则它必须是先前由 `malloc`、`calloc`、`realloc` 或 `reallocarray` 返回的有效指针，且尚未被 `free` 或 `realloc` 释放
- 无其它前置条件；`m` 和 `n` 可以为任意 `usize` 值

## 后置条件

**Case 1: `m * n` 乘法溢出 `usize`**

- 若 `n != 0` 且 `m > core::usize::MAX / n`（等价 C 条件: `n && m > -1 / n`），触发溢出
- errno 被设置为 `ENOMEM`
- 返回 `core::ptr::null_mut()`
- `ptr` 指向的原始内存块保持未修改状态（未被释放）

**Case 2: 无溢出，`realloc(ptr, m * n)` 成功**

- 返回指向新分配内存块（至少 `m * n` 字节）的指针
- 若 `ptr` 非 null 且 `m * n > 0`，新块内容在 `min(oldsize, m * n)` 范围内与原始块一致
- 若 `ptr` 非 null 且 `m * n == 0`，行为等价于 `free(ptr)` 并可能返回 null 或唯一指针（由内部 `realloc` 决定 exact behavior）
- 若 `ptr` 为 null，行为等价于 `malloc(m * n)`

**Case 3: 无溢出，但 `realloc(ptr, m * n)` 失败**

- 返回 `core::ptr::null_mut()`
- errno 由 `realloc` 内部设置为 `ENOMEM`
- `ptr` 指向的原始内存块保持有效且未修改

## 不变量

- 本函数不持有任何全局锁
- 不修改任何全局或静态状态（除 errno 以外）
- 无论成功或失败，`ptr` 参数的生命周期管理由调用方负责

## 算法

Rust 实现逻辑（等价于 C 源码，可直接在 `extern "C" fn` 内实现）：

```
1. 溢出检测: 利用 usize 无符号算术特性
   - 表达式 if n != 0 && m > core::usize::MAX / n
   - 当 n == 0 时，Rust 短路求值避免除以零
   - 当 n == 0 时不进行溢出检查，交由 realloc(ptr, 0) 处理

2. 若溢出:
   - unsafe { *__errno_location() = ENOMEM; }
   - 返回 core::ptr::null_mut()

3. 否则:
   - 调用 realloc(ptr, m * n) 将结果透传给调用者
   - 注意: m * n 在无溢出保证下为安全精确乘法
```

**Rust 设计要点**:
- 溢出检测完全在 safe Rust 中完成（无 unsafe 代码）
- 仅 errno 写入和 realloc 调用需要 `unsafe` 块
- 乘法 `m * n` 在编译时为 debug 模式开启溢出检查（`overflow-checks`），在 release 模式下使用 wrapping 语义（与 C 的无符号溢出回绕一致）但不影响正确性，因为溢出已被显式检测
- 为明确防止 debug 模式下的 panic，建议使用 `m.wrapping_mul(n)` 或在溢出检查通过后再执行乘法

---

## 依赖图

```
reallocarray (External, extern "C", ABI 兼容)
  └── realloc (External, extern "C", rusl::malloc::realloc)
        └── (见 src/malloc/rust-spec/realloc.md 递归依赖树)
  └── __errno_location (External, extern "C", rusl::errno)
```

---

## 符号归类

### 对外导出符号 (External)

| 符号 | Rust 签名 | 说明 |
|------|-----------|------|
| `reallocarray` | `pub unsafe extern "C" fn reallocarray(ptr: *mut c_void, m: usize, n: usize) -> *mut c_void` | BSD 扩展函数，必须保持 C ABI 兼容 |

### 外部依赖符号 (External -- 同 crate 其他模块提供)

| 符号 | 来源 | 说明 |
|------|------|------|
| `realloc` | `rusl::malloc::realloc` | C ABI libc 函数，实际内存重分配 |
| `__errno_location` | `rusl::errno` | 返回线程局部 errno 的指针 `*mut c_int` |
| `ENOMEM` | `rusl::errno` | 表示内存不足的错误码常量 (Linux 上通常为 12) |

---

## 模块建议

由于 `reallocarray` 逻辑极简（仅一行溢出检查和一次委托调用），不需要拆分为多个模块。直接在单一文件中实现：

```
rusl/src/malloc/
  reallocarray.rs    -- 仅包含 extern "C" fn reallocarray，无内部辅助函数
```

---

/* Rely */
[RELY]
Predefined Structures/Functions:
  // 来自 rusl::malloc::realloc (同 crate sibling 模块)
  pub unsafe extern "C" fn realloc(ptr: *mut core::ffi::c_void, size: usize) -> *mut core::ffi::c_void;
                                  // 依赖1: 来自 rusl::malloc::realloc 模块，
                                  //        执行实际的内存重分配操作，A C ABI 兼容的导出符号

  // 来自 rusl::errno (同 crate sibling 模块)
  pub extern "C" fn __errno_location() -> *mut core::ffi::c_int;
                                  // 依赖2: 来自 rusl::errno 模块，
                                  //        返回当前线程的 errno 指针，musl 标准实现

Predefined Macros/Constants:
  pub const ENOMEM: core::ffi::c_int = 12;
                                  // 依赖3: 来自 rusl::errno 模块，
                                  //        Linux 上 ENOMEM 值为 12，表示内存不足错误

Predefined Types:
  core::ffi::c_void               // 依赖4: C void 类型对应的 Rust 不透明类型
  usize                           // 依赖5: Rust 的机器字长无符号整数，ABI 兼容 C size_t
  core::ffi::c_int                // 依赖6: C int 类型，用于 errno 读写

[GUARANTEE]
Exported Interface:
  // 本模块保证对外提供的接口签名，必须满足 C ABI 兼容性
  #[no_mangle]
  pub unsafe extern "C" fn reallocarray(ptr: *mut core::ffi::c_void, m: usize, n: usize) -> *mut core::ffi::c_void;
  // BSD 扩展函数: 带溢出检查的安全数组内存重分配
  // - 声明于 <stdlib.h> (C 侧)，受 _BSD_SOURCE 特性测试宏保护
  // - m * n 溢出时: errno = ENOMEM, 返回 null, ptr 保持有效
  // - 无溢出时: 等价于 realloc(ptr, m * n)，透传其返回值和 errno
  // - ptr = null 时: 等价于 malloc(m * n)