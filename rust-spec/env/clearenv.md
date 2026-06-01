# clearenv.rs 归约

> 源码位置: `src/env/clearenv.rs`
> 标准归属: GNU 扩展（非 POSIX），需定义 `_GNU_SOURCE` 方可使用
> 约束: `#![no_std]`，不依赖 Rust 标准库，不依赖其他 libc

---

## 依赖图

```
clearenv (pub extern "C")
  ├── __ENVIRON          → 模块内部全局 AtomicPtr<c_char>
  │     └── 外部通过 environ 符号 (POSIX 标准全局变量) 可见
  ├── RM_ADD_FN          → 模块内部全局 AtomicPtr<EnvRmAddFn>, 默认指向 no_op
  │     └── 由 setenv 模块通过 register_env_rm_add() 注册真实实现
  └── no_op              → pub(crate) 内部函数 (本模块)
```

**依赖说明**:
- `__ENVIRON`: Rust 内部使用 `AtomicPtr<c_char>` 存储环境变量数组首地址。替代 C 的 `extern char **__environ` 裸指针。未链接 `setenv`/`putenv` 时，其内容由内核/CRT 传递的环境块提供。
- `RM_ADD_FN`: 模块内部 `static AtomicPtr<EnvRmAddFn>`，存储 `__env_rm_add` 的回调函数指针。替代 C 的 `weak_alias(dummy, __env_rm_add)` 弱符号机制。若链接了 `setenv` 模块，则其通过 `register_env_rm_add()` 注册真实的内存管理实现，覆盖默认的 `no_op`；否则回退到 `no_op` 无操作。效果与 C 弱符号等价，但在 Rust 中通过原子操作而非链接器特性实现。
- `no_op`: Rust 内部无操作函数，作为 `RM_ADD_FN` 的默认回调，对应 C 的 `static void dummy(char *old, char *new) {}`。

---

## no_op (内部函数)

```rust
unsafe extern "C" fn no_op(_old: *mut core::ffi::c_char, _new: *mut core::ffi::c_char) {}
```

[Visibility]: **Internal (不导出)** — `pub(crate)` 函数，rusl 内部实现细节，POSIX/C 标准未定义。

### 前置条件
- 无。此函数无任何调用前提条件。

### 后置条件
- Case 1 (总是): 函数体为空，无任何副作用。参数 `_old` 和 `_new` 被忽略，不作任何操作。

### Intent
作为 `RM_ADD_FN` 全局回调指针的默认值，对应 C 的 `dummy` 函数。当程序未链接 `setenv` 模块（即未使用 `setenv`/`putenv`）时，`clearenv` 中对回调的调用将执行此函数。由于此时环境变量字符串来自内核/CRT 传递的原始内存区域，无需（也不能）通过 `free()` 释放，因此无操作是完全正确的。

**Rust 设计说明**: 由于 Rust 不支持 GNU `weak_alias` 扩展，采用 `AtomicPtr<EnvRmAddFn>` 存储回调函数指针来模拟弱符号的可选依赖机制。`no_op` 作为默认回调，`setenv` 模块通过原子交换替换为真实实现。这种设计比 C 弱符号更安全：回调注册由原子操作保护，避免了链接期的不确定性，且在 Rust 类型系统中有明确的类型签名约束。

---

## RM_ADD_FN 全局回调指针

```rust
type EnvRmAddFn = unsafe extern "C" fn(*mut core::ffi::c_char, *mut core::ffi::c_char);

static RM_ADD_FN: AtomicPtr<EnvRmAddFn> = AtomicPtr::new(no_op as *mut EnvRmAddFn);
```

[Visibility]: **Internal (不导出)** — 模块级 `static`，`RM_ADD_FN` 对外不可直接访问（是 rusl 内部实现细节）。外部通过 `pub(crate) fn register_env_rm_add()` 进行注册。

### 前置条件
- 初始化时：`RM_ADD_FN` 被初始化为指向 `no_op` 函数。
- 运行时：`setenv` 模块可在初始化阶段通过 `register_env_rm_add()` 替换为真实的内存管理回调，该操作必须是原子交换以保证与 `clearenv` 调用的同步。

### 后置条件
- Case 1 (setenv 模块已注册): `RM_ADD_FN` 指向 `src/env/setenv.rs` 中注册的实现，该实现维护内部分配字符串的记录表，在替换或清除时调用解注册逻辑并释放旧字符串。
- Case 2 (setenv 模块未注册): `RM_ADD_FN` 保持指向 `no_op`，调用时无任何副作用。

### Intent
通过 `AtomicPtr<EnvRmAddFn>` 实现**可选依赖**，替代 C 的 GNU `weak_alias` 机制。当 `clearenv` 被调用时，若用户曾通过 `setenv`/`putenv` 分配过堆上的环境字符串（由注册的真实回调管理），则这些字符串被正确释放；否则直接忽略，因为 `__ENVIRON` 中存储的原始环境字符串不可被释放。

**与 C 对比**:
| 方面 | C 实现 | Rust 实现 |
|------|--------|-----------|
| 可选依赖机制 | GNU `weak_alias` 链接器特性 | `AtomicPtr` 运行时回调注册 |
| 类型安全 | 弱别名无类型检查 | Rust 函数指针类型签名约束 |
| 并发安全 | 依赖外部同步保证 | 原子操作保证内存顺序 |
| 平台可移植性 | 依赖 GNU 扩展 | 纯标准 Rust + `core::sync::atomic` |

---

## clearenv (对外导出)

```rust
pub extern "C" fn clearenv() -> core::ffi::c_int;
```

[Visibility]: **Public (对外导出)** — GNU 扩展，声明于 `<stdlib.h>`（需定义 `_GNU_SOURCE` 宏）。用户程序可直接调用。ABI 与原 C 接口完全兼容：使用 `extern "C"` 调用约定，返回 `c_int`。

### 前置条件
- `__ENVIRON` 指针可为任意值：
  - 指向 `*mut c_char` 数组（由操作系统/CRT 传递的环境块），以 `core::ptr::null_mut()` 终止
  - 为 `core::ptr::null_mut()`（环境变量已清空）
  - 指向的数组中含有由 `setenv`/`putenv` 通过堆分配后经 `RM_ADD_FN` 回调登记的字符串
- 调用无需外部锁，但多线程环境下并发修改环境变量是**未定义行为**（符合 POSIX 关于 environ 的线程安全限制）。

### 后置条件
- **返回值**: 始终返回 `0`（成功）。
- **状态转换**:
  1. `__ENVIRON` 被原子地设置为 `core::ptr::null_mut()`（空指针），环境变量数组被清空。
  2. 遍历旧的 `__ENVIRON` 数组，对每个非 `NULL` 条目通过 `RM_ADD_FN` 存储的回调函数通知环境修改：
     - 若回调为真实实现（`setenv` 模块已注册）：若 `entry` 在分配记录表中，将其解除登记并释放内存；否则无操作。
     - 若回调为 `no_op`（默认）：无操作。
- **不变量**: 调用后 `__ENVIRON` 为 `core::ptr::null_mut()`，即外部通过 `getenv()` 访问将得到空环境。

### 系统算法

```
算法 clearenv:
  输入: 无
  输出: 0（总是成功）

  1. e := __ENVIRON.load(Acquire)       // 原子加载旧的环境数组指针
  2. __ENVIRON.store(null_mut(), Release) // 原子地立即清空全局环境指针
  3. 若 e ≠ null:
       对于 e 指向的数组中每个非 NULL 元素 s:
         let callback = RM_ADD_FN.load(Acquire)  // 读取当前回调
         callback(s, null_mut())                   // 通知环境修改，可能释放堆内存
         移动到下一个元素
  4. 返回 0
```

设计要点：
- **先清空后释放**: 先将 `__ENVIRON` 置为 `null_mut()`，再遍历旧数组调用回调。这保证了在回调过程中，任何对 `getenv()` 的调用都已看到空环境。使用 `Acquire/Release` 内存顺序确保跨线程可见性。
- **原子指针替代弱符号**: 通过 `AtomicPtr<EnvRmAddFn>` 避免了 `clearenv` 对 `setenv` 模块的硬链接依赖。这也意味着在不使用 `setenv`/`putenv` 的程序中，调用 `clearenv` 不会引入任何堆内存管理相关的代码和开销。
- **不变量强制**: `__ENVIRON` 的修改使用 `store(Release)`，确保在 `setenv`/`putenv` 中对环境的修改不会被重排到清空操作之后。

### Intent
清除当前进程的所有环境变量。这是 `unsetenv` 遍历所有键的批量操作等价物，但其实现利用直接操作 `__ENVIRON` 指针来避免逐键查找的开销。通过 `RM_ADD_FN` 回调机制，确保由 `setenv`/`putenv` 分配的堆内存被正确回收，防止内存泄漏。调用后，外部通过 `getenv()` 访问所有环境变量都将返回 `NULL`，`setenv`/`putenv` 将从空环境开始重新建立环境变量表。

---

## register_env_rm_add (内部注册函数)

```rust
pub(crate) unsafe fn register_env_rm_add(f: EnvRmAddFn);
```

[Visibility]: **Internal (不导出)** — `pub(crate)` 可见性，仅 `rusl` crate 内部 `setenv` 模块可调用。不对外部用户暴露。

### 前置条件
- `f` 必须是一个有效的函数指针，指向具有正确 `EnvRmAddFn` 签名的函数。
- 该函数应在 `setenv` 模块初始化期间调用一次，多次调用的行为取决于具体实现（通常是最后注册者生效）。
- 调用者必须确保 `f` 指向的代码在整个程序生命周期内保持有效。

### 后置条件
- `RM_ADD_FN` 被原子地替换为 `f`，使用 `store(Release)` 以保证后续 `clearenv` 调用能通过 `load(Acquire)` 观测到此更新。
- 旧的 `RM_ADD_FN` 值被丢弃（不再可恢复）。

### Intent
提供内部注册入口，允许 `setenv` 模块将其内存管理回调注入到环境变量模块中。这是 Rust 中替代 C 弱符号的方式：不是依赖链接器的符号覆盖行为，而是通过显式的运行时注册来建立模块间的依赖关系。

---

/* Rely */
[RELY]
Predefined Structures/Functions:
  core::sync::atomic::AtomicPtr<EnvRmAddFn>  // 依赖1: 原子函数指针，实现可选依赖的回调注册
  core::sync::atomic::Ordering               // 依赖2: 内存顺序（Acquire/Release），保证环境修改的跨线程可见性
  core::ffi::c_char                          // 依赖3: C 字符类型，用于环境字符串指针参数
  core::ffi::c_int                           // 依赖4: C int 类型，用于返回值

Predefined Macros/Traits:
  (none)

[GUARANTEE]
Exported Interface:
  pub extern "C" fn clearenv() -> core::ffi::c_int;
    // 本模块保证对外提供的 C ABI 兼容接口

Internal Interface (仅 rusl crate 内部可见，不出现在最终 .so/.a 的导出符号表中):
  pub(crate) unsafe fn register_env_rm_add(f: EnvRmAddFn);
    // 本模块提供给 setenv 模块的回调注册接口
