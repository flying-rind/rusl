# \_\_errno_location 函数规约

## 复杂度分级: Level 1

> musl libc 内部 `errno` 线程局部存储访问器实现。返回当前线程的 `errno` 变量地址。
>
> Stage 0 使用全局静态变量，非线程安全；Stage 5 将迁移至 pthread 结构体内部的 per-thread 存储。

---

## 函数接口

```rust
use core::ffi::c_int;

extern "C" fn __errno_location() -> *mut c_int;

// weak_alias: ___errno_location 是 __errno_location 的弱别名，共享同一实现
extern "C" fn ___errno_location() -> *mut c_int;
```

[Visibility]: `__errno_location` 和 `___errno_location` 在 musl 中均为 Internal 符号——不直接对外暴露给用户代码，而是通过 `<errno.h>` 中的宏 `errno`（展开为 `(*__errno_location())`）间接使用。但在编译产物（共享库/静态库）中两者均被 `#[no_mangle]` 导出，以保证 C 侧的链接可见性和 ABI 兼容性。

---

### 前置/后置条件

**[Pre-condition]:**
- `__errno_location()`: 无前置条件。Stage 0 实现无需线程初始化即可调用。
- `___errno_location()`: 同 `__errno_location`，完全相同。

**[Post-condition]:**
- 返回类型为 `*mut c_int`，指向当前"线程"的 `errno` 存储。
- 返回值始终非空（非零地址）。
- 不设置 errno 自身。
- Stage 0: 所有调用返回同一个全局静态地址（非线程安全）。
- Stage 5: 不同线程返回不同的地址（线程安全）。

**[Error Behavior]:**
- 本函数不产生错误。始终成功返回有效指针。

---

### 不变量

**[Invariant]:**
- 对于同一进程内的任意调用，返回值始终指向有效的 `c_int` 内存，该内存可读写。
- `__errno_location()` 和 `___errno_location()` 返回完全相同的地址（二者是同一函数体的别名）。
- Stage 0: 全局 `static mut ERRNO: c_int` 是唯一的 errno 存储，所有线程共享。
- 未来 Stage 5: 每个线程拥有独立的 errno 存储，存储于 pthread 控制块 `errno_val` 字段中。

---

### 意图

提供当前线程 errno 变量的地址访问入口。用户代码不直接调用此函数，而是通过 `<errno.h>` 中定义的 `errno` 宏来读写当前线程的错误码。

Rust 侧实现：
- Stage 0 使用全局 `static mut ERRNO: c_int = 0`，通过 `core::ptr::addr_of_mut!(ERRNO)` 获取地址。
- 同时提供 `___errno_location` 作为 GNU 兼容的弱别名，与 `__errno_location` 共享相同的函数体（在 Rust 侧通过复制函数体实现，因 Rust 无直接的 weak_alias 宏；或使用 `#[link_name]` / linker 脚本实现真正的弱符号别名）。
- `set_errno(val)` 为内部辅助函数，调用 `__errno_location()` 获取指针后写入，供其他模块设置 errno 使用。

### 系统算法

```
__errno_location():
  // Stage 0 (当前实现)
  return core::ptr::addr_of_mut!(ERRNO)  // 全局静态变量地址

  // Stage 5 (目标实现)
  // return &__pthread_self()->errno_val

___errno_location():
  同 __errno_location() 的函数体
```

时间复杂度 O(1)。

---

## 依赖图

```
__errno_location    (Stage 0: 无外部依赖)
___errno_location   (等价于 __errno_location)
```

Stage 0 无外部模块依赖。未来 Stage 5 将增加依赖 `__pthread_self()`。

---

## [RELY]

无外部依赖（Stage 0）。

## [GUARANTEE]

Exported Interface:
  `extern "C" fn __errno_location() -> *mut c_int;`
  `extern "C" fn ___errno_location() -> *mut c_int;`

本模块保证对外提供上述两个 ABI 兼容的函数符号，行为符合 POSIX errno 访问语义。___errno_location 与 __errno_location 行为完全一致，均为弱别名关系。