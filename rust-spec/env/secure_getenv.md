# secure_getenv.rs 规约

## 依赖图

```
secure_getenv
  ├── LIBC_SECURE (依赖 — 来自 src/internal/libc_state.rs，rusl 内部全局原子标志)
  └── getenv       (依赖 — 来自 src/env/ 模块，POSIX 标准函数)
```

---

## 原始 C 接口

```c
char *secure_getenv(const char *name);
```

**[Visibility]: Public** -- GNU 扩展函数（需 `_GNU_SOURCE`），声明于 `<stdlib.h>`。用户程序可直接调用。

---

## 内部依赖: 安全模式标志

### 原始 C 设计

```c
// 来自 src/internal/libc.h，musl 内部全局状态
struct __libc {
    // ...其他字段...
    char secure;   // 进程安全模式标志，取值为 0 或 1
    // ...其他字段...
};
```

**[Visibility]: Internal (不导出)** -- musl 内部使用的全局库状态结构体，用户代码不可直接访问。

### Rust 重设计

在 Rust 实现中，`libc.secure` 字段定义于 `src/internal/libc_state.rs`：

```rust
// src/internal/libc_state.rs

// 方式一: 在 LibcState 结构体中 (repr(C), 与 C 结构体 ABI 兼容)
#[repr(C)]
pub(crate) struct LibcState {
    pub(crate) secure: bool,  // char -> bool, repr(C) 下 C char 与 Rust bool 布局兼容
    // ...其他字段...
}

// 方式二: 独立原子标志，由 __init_libc 在启动阶段设置，与 LibcState.secure 保持同步
//         提供无锁读取接口，使 secure_getenv 等模块无需访问完整 LibcState 结构体
pub(crate) static LIBC_SECURE: core::sync::atomic::AtomicBool =
    core::sync::atomic::AtomicBool::new(false);
```

**[Visibility]: Internal** -- `pub(crate)`，仅 rusl 内部使用。

**设计说明:**

- `LibcState.secure` 与 `LIBC_SECURE` 在 `__init_libc` 中同步设置，二者语义等价。
- `LIBC_SECURE` 作为独立 `AtomicBool` 提供无锁读取，`secure_getenv` 无需访问完整 `LibcState` 结构体即可判断安全模式。
- 使用 `AtomicBool` 替代 C 的 `char` 标志，提供清晰的布尔语义，且 `load(Ordering::Relaxed)` 在大多数平台上编译为普通内存读取指令，与 C 原版开销完全相同。

**语义:**

- 当进程以特权模式（如 setuid/setgid 二进制）运行时，musl 启动代码 `__init_libc` 将其设置为 `true`。
- 该字段用于控制是否允许危险的环境变量操作（如 `LD_PRELOAD`、`LD_LIBRARY_PATH`），以防止特权升级攻击。

**不变量 (Invariant):**

- `LIBC_SECURE` 与 `LibcState.secure` 的值始终一致，在 `__init_libc` 中同时设置。
- 一旦在进程启动期被设置（从 `__init_libc` 中依据 `AT_SECURE` 辅助向量或 uid/gid 检测），在整个进程生命周期内保持不变。
- 由于初始化完成之后仅读取、不写入，在多线程环境中读取是安全的（无需加锁）。

---

## Rust 外部 ABI 接口

```rust
// [Visibility]: Public — GNU 扩展函数，用户程序可直接调用
// 对应于 glibc/musl 的 char *secure_getenv(const char *name);
#[no_mangle]
pub extern "C" fn secure_getenv(name: *const core::ffi::c_char) -> *mut core::ffi::c_char;
```

**ABI 兼容性说明:**

| C 签名 | Rust 签名 |
|--------|-----------|
| `char *secure_getenv(const char *name)` | `pub extern "C" fn secure_getenv(name: *const c_char) -> *mut c_char` |

- 参数 `name: *const c_char` 对应 `const char *name`，使用常量指针语义。
- 返回值 `*mut c_char` 对应 `char *`，调用者只能读取返回的内存，不应修改或释放。
- `#[no_mangle]` + `extern "C"` 确保该符号的 ABI 与 C 版本完全兼容，外部 C 代码可透明调用。

**关于函数签名中是否使用 `unsafe`:**

- 该函数自身不包含需要调用者保证安全不变量的操作（不解引用裸指针写入、不修改全局可变状态、不调用不安全的系统调用）。主体逻辑为读取 `AtomicBool` + 条件分支 + 调用 `getenv`。
- 因此函数签名**不标记为 `unsafe extern "C"`**。调用者传入无效指针的风险由 `getenv` 内部承担，而非本函数的职责。
- 函数体内的 `getenv(name)` 调用同样不要求 `unsafe` 块（`getenv` 自身已在内部处理 `environ` 的 `static mut` 访问）。

---

## 意图 (Intent)

提供安全的环境变量访问：在特权执行上下文（如 setuid/setgid 进程）中，拒绝返回环境变量值以防止环境变量注入攻击。在普通上下文中，等价于 `getenv`。

---

## 前置条件 (Preconditions)

| 条件 | 说明 |
|------|------|
| `name != core::ptr::null()` | 调用者必须传入指向以 NUL 结尾的 C 字符串的有效指针 |
| `*name` 是以 `'\0'` 结尾的合法 C 字符串 | 标准 C 字符串约束 |
| `name` 指向的字符串中不包含 `'='` 字符 | POSIX 标准规定：环境变量名不得包含 `=`；若含 `=`，`getenv` 返回 `null_mut()` |
| 全局 `LIBC_SECURE` 已在进程启动时正确初始化 | 由 `__init_libc` 在 `main()` 之前设置 |

---

## 后置条件 (Postconditions)

**Case 1 -- 安全模式（`LIBC_SECURE.load(Relaxed) == true`）:**

| 条件 | 结果 |
|------|------|
| `LIBC_SECURE == true` | 进程处于特权执行上下文（setuid/setgid） |
| 返回值 | 直接返回 `core::ptr::null_mut()`，不访问环境变量列表 |
| 副作用 | 无论 `name` 是否匹配任何环境变量，均返回 `null_mut()` |
| 线程安全性 | 仅读取 `AtomicBool`，无写入操作，天然线程安全 |

**Case 2 -- 普通模式（`LIBC_SECURE.load(Relaxed) == false`）:**

| 条件 | 结果 |
|------|------|
| `LIBC_SECURE == false` | 进程处于普通执行上下文 |
| 行为 | 等价于调用 `getenv(name)` |
| 返回值 | 与 `getenv(name)` 完全一致: 若 `name` 匹配某个环境变量，返回指向该环境变量值字符串的指针（如 `"PATH=/usr/bin"` 中 `=` 之后的部分）；若未匹配，返回 `core::ptr::null_mut()` |
| 线程安全性 | 与 `getenv` 相同：读操作本身无数据竞争，但若其他线程同时调用 `putenv`/`setenv`/`unsetenv`/`clearenv`，行为未定义 |

---

## 系统算法 (System Algorithm)

```
fn secure_getenv(name: *const c_char) -> *mut c_char:
    // Step 1: 读取安全模式标志
    //  使用 Ordering::Relaxed —— 该标志仅在启动期（单线程上下文）写入一次，此后只读
    //  Relaxed 语义在 x86_64、aarch64 等平台上零额外开销
    if LIBC_SECURE.load(core::sync::atomic::Ordering::Relaxed) {
        // Case 1: 安全模式 —— 拒绝所有环境变量访问
        return core::ptr::null_mut();
    }

    // Case 2: 普通模式 —— 委托给标准 getenv
    //  name 的合法性检查由 getenv 内部完成 (NULL 检查、空字符串检查、'=' 检测)
    return getenv(name);
```

**算法要点:**

1. **安全性优先**: 该函数在条件判断中使用 `LIBC_SECURE`（而非执行额外的 uid/gid 检查），因为 `LIBC_SECURE` 的初始化已经综合了 `AT_SECURE` 辅助向量和实际 uid/gid 比较（由 `__init_libc` 完成）。这避免了竞态条件和冗余系统调用。

2. **惰性检查**: 首先检查 `LIBC_SECURE`，在安全模式下尽早返回，避免无谓的 `getenv` 调用开销。

3. **委托模式**: 在普通模式下直接调用 `getenv(name)`，复用其完整的名称解析逻辑（包括名称校验、环境数组遍历、匹配算法），不重复实现。

4. **零开销抽象**: `AtomicBool::load(Relaxed)` 在大多数平台上编译为普通内存读取指令，与 C 原版的 `libc.secure` 字段读取开销完全相同。

5. **线程安全**: 该函数仅读取 `AtomicBool`（只读原子变量）和调用 `getenv`（读 `environ`），无写入操作，天然线程安全。

---

## 不变量 (Invariants)

- `secure_getenv` 返回的指针在下次修改环境的调用（`putenv`、`setenv`、`unsetenv`、`clearenv`）之前保持有效；修改环境后，该指针可能指向已被替换或释放的内存。
- 若多次以相同 `name` 调用 `secure_getenv` 且期间未修改环境且 `LIBC_SECURE` 未变，每次返回值相同（等于 `getenv(name)` 的返回值）。
- 该函数不设置 `errno`。
- 在安全模式下，`secure_getenv` 一定返回 `core::ptr::null_mut()`，无论 `name` 内容为何。

---

## GNU 兼容性

`secure_getenv` 是 glibc 引入的 GNU 扩展，musl 提供兼容实现。rusl 的 Rust 实现通过 `#[no_mangle] pub extern "C"` 保持 ABI 兼容，确保以下场景透明工作：

- 用户代码通过 `<stdlib.h>` 声明调用（配合 `_GNU_SOURCE` 特性宏）。
- 动态链接的二进制文件在 musl 和 glibc 之间切换时，该符号的 ABI 不变。
- 静态链接时，链接器可直接解析 `secure_getenv` 符号。

---

## 与原有 C 实现的关键差异

| 方面 | C 实现 (musl) | Rust 实现 (rusl) |
|------|---------------|------------------|
| 安全标志类型 | `struct __libc { char secure; }` | `AtomicBool` (模块级静态变量) |
| 安全标志读取 | 直接读取结构体字段 `libc.secure` | `LIBC_SECURE.load(Relaxed)` |
| 函数声明 | `char *secure_getenv(const char *)` | `pub extern "C" fn secure_getenv(name: *const c_char) -> *mut c_char` |
| `unsafe` 范围 | C 无 unsafe 概念 | 函数签名不标记 `unsafe`；内部无 `unsafe` 块（`getenv` 自行管理不安全边界） |
| 内部可见性 | C 文件级 `static` 无额外内部符号 | `getenv` 为 `pub(crate)`，`LIBC_SECURE` 为 `pub(crate)` |
| 返回值语义 | 完全一致 | 完全一致 |

---

## 内部实现设计要点

- `LIBC_SECURE` 作为 `AtomicBool` 存储在 `src/internal/libc_state.rs` 中，`secure_getenv` 通过 `crate::internal::libc_state::LIBC_SECURE` 路径访问。
- 使用 `Ordering::Relaxed` 读取，因为该标志仅在启动阶段（单线程上下文）写入一次，之后所有线程只读。Relaxed 语义在 x86_64、aarch64 等平台上零额外开销。
- `getenv` 来自 `src/env/getenv.rs`，作为模块公共函数（`pub(crate)` 可见性即可）。Rust 内部可直接调用而不需要额外的 FFI 边界。
- 函数的控制流简单（一个 `if` 分支），无需复杂的状态机或错误处理路径。
- 在 `__init_libc` 中，`LibcState.secure` 和 `LIBC_SECURE` 必须同步设置，确保二者始终一致。具体而言，`__init_libc` 在检测到 SUID/SGID 安全执行模式时，执行：
  ```rust
  unsafe { LIBC_STATE.secure = true; }
  LIBC_SECURE.store(true, Ordering::Release);
  ```

---

```
/* Rely */

[RELY]
Predefined Structures/Functions:

  // 依赖1: LIBC_SECURE — 来自 src/internal/libc_state.rs
  //   进程安全模式原子标志，在 __init_libc 中设置为 true/false。
  //   使用 AtomicBool 实现，提供无锁读取。
  //   [Visibility]: Internal — pub(crate)
  pub(crate) static LIBC_SECURE: core::sync::atomic::AtomicBool;

  // 依赖2: getenv — 来自 src/env/getenv.rs
  //   POSIX 标准环境变量查找函数。
  //   在进程环境变量列表中查找指定名称的环境变量并返回其值字符串指针。
  //   [Visibility]: Public
  pub extern "C" fn getenv(name: *const core::ffi::c_char) -> *mut core::ffi::c_char;

Predefined Types:
  core::ffi::c_char                              // C 字符类型，用于函数参数和返回值
  core::sync::atomic::AtomicBool                 // 原子布尔类型，用于无锁安全标志
  core::sync::atomic::Ordering                   // 内存排序枚举，用于原子操作排序语义

/* Guarantee */

[GUARANTEE]
Exported Interface:

  // 函数: secure_getenv (GNU 扩展，对外导出)
  #[no_mangle]
  pub extern "C" fn secure_getenv(name: *const core::ffi::c_char) -> *mut core::ffi::c_char;
  //   安全地获取环境变量值。
  //   - name: 要查找的环境变量名称（C 字符串，不得为 NULL）
  //   - 安全模式 (LIBC_SECURE == true): 始终返回 core::ptr::null_mut()
  //   - 普通模式 (LIBC_SECURE == false): 等价于 getenv(name)，返回指向值字符串的指针
  //   - 返回的指针指向进程环境内存，调用者不可修改或释放
  //   - 该函数不设置 errno
  //   - 本模块保证上述签名和语义在所有支持的平台上保持 ABI 兼容
```
