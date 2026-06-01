# putenv.rs 规约

> 源码位置: `src/env/putenv.rs`
> 标准归属: POSIX.1-2001, SVr4, 4.3BSD; 声明于 `<stdlib.h>`
> 约束: `#![no_std]`，不依赖 Rust 标准库，不依赖其他 libc

---

## 依赖图

```
putenv (pub unsafe extern "C", #[no_mangle])
  ├── __strchrnul    → see src/string/rust-spec/__strchrnul.md (跨模块)
  ├── unsetenv       → see src/env/rust-spec/unsetenv.md (跨模块)
  └── putenv_core (Internal, pub(crate))
        ├── __environ     → see src/env/rust-spec/__environ.md (跨模块)
        ├── 内部全局分配器   → alloc::alloc::Global 等 (Rust core::alloc, 替代 malloc/realloc/free)
        ├── strncmp 等价实现 → 内部字节比对，替代 <string.h> strncmp (no_std)
        ├── memcpy 等价实现  → core::ptr::copy_nonoverlapping, 替代 <string.h> memcpy (no_std)
        └── ENV_RM_ADD     → 静态 AtomicPtr<EnvRmAddFn>, 默认指向 no_op (替代 C 弱符号 __env_rm_add)
              └── no_op (module-private, 空实现)
```

**依赖说明**:
- `__strchrnul`: 跨模块内部函数，用于在 `s` 中查找 `=` 位置。在 Rust 中可替换为 `core::slice::memchr` 或手动字节遍历，但现阶段保持跨模块依赖以与原架构一致。
- `unsetenv`: 跨模块公共函数，当 `s` 不含 `=` 时委托其处理移除操作。
- `__environ`: 环境变量数组的全局指针。Rust 内部以 `AtomicPtr<c_char>` 存储，替代 C 的 `extern char **environ`。
- `ENV_RM_ADD`: 模块内部 `static AtomicPtr<EnvRmAddFn>`，存储 `__env_rm_add` 的回调函数指针。替代 C 的 `weak_alias(dummy, __env_rm_add)` 弱符号机制。若链接了 `setenv` 模块，则其通过 `register_env_rm_add()` 注册真实的内存管理实现；否则回退到 `no_op` 空操作。

---

## no_op (内部辅助函数)

```rust
unsafe extern "C" fn no_op(_old: *mut core::ffi::c_char, _new: *mut core::ffi::c_char) {}
```

[Visibility]: **Internal (不导出)** -- `pub(crate)` 函数，rusl 内部实现细节，对应 C 的 `static void dummy(char *old, char *new) {}`。

### 意图 (Intent, Level 2)

`no_op` 是一个空占位函数，作为 `ENV_RM_ADD` 回调指针的默认值。它的语义是"什么都不做"：当 musl 的 `putenv` / `unsetenv` 被调用且 `setenv` 模块未被链接/初始化时（即未注册真实的内存管理回调），环境变量被替换/移除时不需要额外的内存管理 Hook。这使得 `putenv` 和 `unsetenv` 模块可独立运行，而不会因缺少 `__env_rm_add` 符号而失败。

**Rust 设计说明**: 由于 Rust 不支持 GNU `weak_alias` 扩展，采用 `AtomicPtr<EnvRmAddFn>` 存储回调函数指针来模拟弱符号的可选依赖机制。`no_op` 作为默认回调，`setenv` 模块通过原子交换替换为真实实现。这种设计比 C 弱符号更安全：回调注册由原子操作保护，避免了链接期的不确定性，且在 Rust 类型系统中有明确的类型签名约束。

### 前置条件

- 无前置条件。`_old` 和 `_new` 可以为任意值（包括 null），函数体为空，不访问参数。

### 后置条件

- 无任何副作用，返回值类型为 `()`，不做任何操作。

---

## ENV_RM_ADD 全局回调指针

```rust
type EnvRmAddFn = unsafe extern "C" fn(*mut core::ffi::c_char, *mut core::ffi::c_char);

static ENV_RM_ADD: core::sync::atomic::AtomicPtr<EnvRmAddFn>
    = core::sync::atomic::AtomicPtr::new(no_op as *mut EnvRmAddFn);
```

[Visibility]: **Internal (不导出)** -- 模块级 `static`，`ENV_RM_ADD` 对外不可直接访问（是 rusl 内部实现细节）。外部通过 `pub(crate) fn register_env_rm_add()` 进行注册。

### 意图 (Intent, Level 2)

`ENV_RM_ADD` 是 musl 环境变量模块的内部通信协议：它是一个可选的回调函数指针，用于在环境变量被替换（`putenv`/`putenv_core`）或移除（`unsetenv`）时通知内存管理模块。`setenv` 模块在初始化时将 `ENV_RM_ADD` 设置为自己的处理函数，该函数追踪所有通过 `setenv` 分配的环境字符串，以便在后续 `setenv` 或 `unsetenv` 调用时正确释放（`dealloc(old)`）和更新跟踪表。若调用方只使用 `putenv` 和 `unsetenv`（不涉及 `setenv` 的堆分配），则该 Hook 保持为 `no_op`，无需执行任何操作。

### 前置条件

- 初始化时：`ENV_RM_ADD` 被初始化为指向 `no_op` 函数。
- 运行时：`setenv` 模块可在初始化阶段通过 `register_env_rm_add()` 替换为真实的内存管理回调。

### 后置条件

**默认行为（`no_op`）：**
- 调用时无操作，忽略 `old` 和 `new`

**当 `setenv` 模块注册后：**
- 若 `old != null` 且在跟踪表中，则：将其替换为 `new`，并 `dealloc(old)`
- 若 `old` 不在跟踪表中且 `new != null`：将 `new` 添加到跟踪表，以便后续释放
- 若 `old` 不在跟踪表中且 `new == null`：无操作（`old` 不是由 `setenv` 分配的，无需释放）
- 不变量：跟踪表始终只包含当前有效的、由 `setenv` 分配的环境字符串指针

---

## putenv_core (内部核心函数)

```rust
pub(crate) unsafe fn putenv_core(s: *mut core::ffi::c_char, l: usize, r: *mut core::ffi::c_char) -> core::ffi::c_int;
```

[Visibility]: **Internal (不导出)** -- `pub(crate)` 可见性，musl 内部函数，POSIX/C 标准未定义。被 `putenv` 和 `setenv` 调用。对应 C 的 `int __putenv(char *s, size_t l, char *r)`。

### 意图 (Intent, Level 2)

`putenv_core` 是环境变量设置的核心实现。它将字符串 `s` 插入进程的 `__environ` 数组（若 `s` 对应的变量已存在则原地替换），并在必要时扩增 `__environ` 数组的容量。参数 `r` 用于内存管理：若调用方（如 `setenv`）在堆上分配了新字符串，`r` 指向该新字符串；`putenv` 则传入 `r = null_mut()` 表示无需额外内存管理。当环境数组扩容成功但 `r != null` 时，`ENV_RM_ADD` 回调被调用以将新字符串注册到跟踪表。

与 C 版本的主要差异：使用 Rust 全局分配器接口（`core::alloc`）替代 `malloc`/`realloc`/`free`；使用 `core::ptr` 模块进行指针操作；长度参数 `l` 使用 `usize` 类型。

### 参数说明

| 参数 | 类型                     | 含义 |
|------|--------------------------|------|
| `s`  | `*mut core::ffi::c_char` | 指向 `"NAME=VALUE"` 格式的字符串，将被直接放入环境数组（非拷贝） |
| `l`  | `usize`                  | 环境变量名的长度（不含 `=`），即 `=` 在 `s` 中的偏移量 |
| `r`  | `*mut core::ffi::c_char` | 若调用方在堆上分配了字符串，则为指向该内存的指针；否则为 `null_mut()` |

### 前置条件

- `s` 必须指向一个有效的、以 `=` 分隔的 `"NAME=VALUE"` 字符串，且 `*s.add(l) == b'='`
- `l > 0`（变量名非空）
- `__environ` 可能是 `null_mut()`（表示环境变量数组尚未初始化）或指向以 `null_mut()` 结尾的 `*mut c_char` 数组
- `s` 指向的内存生命期必须不短于其在环境数组中的存留时间（调用方负责管理）

### 后置条件

**Case 1: 变量已存在（成功替换）**

- 返回值: `0`
- 在 `__environ` 中查找第一个 `*e` 满足 `s[..l+1] == (*e)[..l+1]`（即前 `l+1` 个字节匹配，含 `=`）
- 将 `*e` 替换为 `s`（原地替换，不改变数组大小）
- 调用 `ENV_RM_ADD` 回调 `(tmp, r)` 通知旧值 `tmp` 被替换为新值 `r`（对于 `putenv` 调用路径，`r = null_mut()`，通知模块旧值被移除；对于 `setenv` 路径，同时注册 `r`）

**Case 2: 变量不存在（需要插入）**

- 返回值: `0` 成功，`-1` 失败（OOM）
- 计算新数组大小: `i+2`（`i` 个现有变量 + 1 个新变量 + 1 个 NULL 终止符）
- 若 `__environ == OLDENV`（上次由 `putenv_core` 分配），使用 `alloc::alloc::realloc` 扩容，失败则跳转 OOM
- 否则使用 `alloc::alloc::alloc` 分配新数组，若 `i > 0` 则 `core::ptr::copy_nonoverlapping` 复制旧内容，释放 `OLDENV`
- 将 `newenv[i] = s`，`newenv[i+1] = null_mut()`，更新 `__environ = OLDENV = newenv`
- 若 `r != null_mut()`，调用 `ENV_RM_ADD` 回调 `(null_mut(), r)` 注册新分配的字符串
- OOM 路径: 若 `r` 非 null，`alloc::alloc::dealloc` 释放调用方传入的堆分配字符串，返回 `-1`

### 不变量 (Invariants)

- **`OLDENV` 追踪**: `static mut OLDENV: *mut *mut c_char` 记录上一次由 `putenv_core` 分配的数组指针。若当前 `__environ == OLDENV`，说明当前环境数组由此模块管理，可使用 `realloc`；否则说明 `__environ` 指向外部传入的数组（如 `execve` 传入的 `envp`），需新分配。
- **NULL 终止**: 环境数组始终以 NULL 指针终止，即 `__environ` 指向的数组的最后一个有效元素之后是 `null_mut()`。
- **`OLDENV` 一致性**: 每次通过分配器分配新数组后，`OLDENV` 始终等于 `__environ`，确保下次插入时能正确识别为"自管理"数组而使用 `realloc`。

---

## putenv (对外导出)

```rust
#[no_mangle]
pub unsafe extern "C" fn putenv(s: *mut core::ffi::c_char) -> core::ffi::c_int;
```

[Visibility]: **Public (对外导出)** -- POSIX 标准函数，`<stdlib.h>` 声明，用户程序可直接调用。必须保持与 C ABI 完全兼容：使用 `extern "C"` 调用约定，`#[no_mangle]` 保证符号名为 `putenv`，参数和返回值类型布局与 C 一致。标记为 `unsafe` 因为调用方必须保证传入的指针 `s` 有效且满足前置条件。

### 系统算法 (System Algorithm, Level 3)

`putenv` 的职责是将调用方提供的 `"NAME=VALUE"` 格式字符串直接放入进程环境（非拷贝）。POSIX 标准规定：调用方不得在 `putenv` 后修改或释放 `s`，且该字符串将作为环境的一部分，直到被后续 `putenv` / `setenv` 覆盖或 `unsetenv` 移除。

完整的执行流程：

1. **解析变量名长度**: 调用 `__strchrnul(s, b'=' as c_int)` 查找 `=` 的位置，计算 `l = 偏移量 = 变量名长度`
2. **有效性检查**: 若 `l == 0`（空变量名）或 `*s.add(l) == 0`（无 `=`），则该字符串不符合 `"NAME=VALUE"` 格式，委托给 `unsetenv(s)` 处理（POSIX 允许此行为：若字符串不包含 `=`，则视为移除同名环境变量）
3. **委托核心逻辑**: 调用 `putenv_core(s, l, null_mut())`，其中 `r = null_mut()` 表示 `putenv` 未在堆上分配字符串，无需 `ENV_RM_ADD` 注册

### 参数说明

| 参数 | 类型                     | 含义 |
|------|--------------------------|------|
| `s`  | `*mut core::ffi::c_char` | 指向 `"NAME=VALUE"` 格式的字符串，调用方必须保证其在整个环境存续期间有效且不被修改 |

### 前置条件

- `s` 不为 null，且指向以 null 结尾的 C 字符串
- 若 `s` 包含 `=`，则 `=` 前至少有一个字符（即 `s[0] != b'='`），否则视为无效，触发 `unsetenv` 行为
- 调用方在 `putenv` 返回后不得修改或释放 `s` 指向的内存，除非之后通过另一次 `putenv`/`setenv` 覆盖或 `unsetenv` 移除了该变量
- 进程的环境变量数组（`environ`/`__environ`）可能处于未初始化状态或已初始化状态

### 后置条件

**Case 1: 设置/替换环境变量（`s` 含 `=`）**

- 返回值: `0`
- `s` 被直接放入环境变量数组（非拷贝），`__environ` 中对应条目指向 `s`
- 若同名变量已存在，旧值被 `s` 取代
- 若同名变量不存在，环境数组扩容并追加 `s`

**Case 2: 移除环境变量（`s` 不含 `=` 或变量名为空）**

- 返回值: `unsetenv(s)` 的返回值
  - `0` 成功（变量被移除或本就不存在）
  - `-1` 且 `errno = EINVAL`（`s` 中包含 `=` 后的字符，即 `l > 0` 但 `*s.add(l) != 0`——实际上此路径几乎不可能触发，因为 `*s.add(l) == 0` 时 `putenv` 本身已检测到无 `=` 并调用 `unsetenv`）

**错误路径 (OOM)**

- `putenv_core` 内部若分配器操作失败，返回 `-1`，此时 `s` 未被添加到环境，原环境不变

### 不变量 (Invariants)

- `putenv` 不拥有 `s` 的内存所有权，因此不负责释放 `s`
- `putenv` 调用通过 `r = null_mut()` 告知 `ENV_RM_ADD` 不注册新内存，这与 `setenv`（传入 `r = s`，注册堆分配内存）形成对比

---

## 与 setenv / unsetenv 的协作语义

`putenv`、`setenv`、`unsetenv` 三个模块通过 `ENV_RM_ADD` 回调指针实现协同：

- **putenv**: 调用 `putenv_core(s, l, null_mut())`，`r = null_mut()`，不注册内存。当替换/移除时通过回调 `(old, null_mut())` 通知旧值被移除。
- **setenv**: 先通过全局分配器构造 `"NAME=VALUE"` 字符串，再调用 `putenv_core(s, l, s)`，`r = s`，在插入成功后通过回调 `(null_mut(), r)` 注册。替换时通过回调 `(old, s)` 释放旧值并注册新值。
- **unsetenv**: 直接遍历 `__environ`，对匹配项通过回调 `(*e, null_mut())` 通知释放，并压缩数组。

这种设计使得 `setenv` 分配的堆内存能在被覆盖或移除时正确释放，而 `putenv` 传入的用户内存不受影响。

---

## Rust 与 C 实现差异说明

| 项目 | C 实现 | Rust 实现 |
|------|--------|-----------|
| 弱符号机制 (`__env_rm_add`) | GNU `weak_alias` 链接器特性 | `AtomicPtr<EnvRmAddFn>` 运行时回调注册 |
| 类型安全 | 弱别名无类型检查 | Rust 函数指针类型签名约束 |
| 并发安全 | 依赖外部同步保证 | 原子操作保证内存顺序 |
| 平台可移植性 | 依赖 GNU 扩展 | 纯标准 `core::sync::atomic` |
| 内存分配 | `malloc` / `realloc` / `free` | `core::alloc` 全局分配器接口 |
| 分配策略判定 | `static char **oldenv` | `static mut OLDENV: *mut *mut core::ffi::c_char` |
| 内存拷贝 | `memcpy` | `core::ptr::copy_nonoverlapping` |
| 字符串比较 | `strncmp` | 内部手动字节遍历或 `core::slice` 方法 |
| 外部未初始化环境 | `extern char **environ` 首次 NULL | `__environ` 初始为 `null_mut()`，首次插入时分配 |
| 内部函数可见性 | 文件内 `static` 或跨模块 `__` 前缀 | `pub(crate)` 或 `pub(self)` |
| `__strchrnul` | 跨模块内部函数调用 | 跨模块内部函数调用（可替换为 `core::slice::memchr`） |

---

/* Rely */
[RELY]
Predefined Structures/Functions (跨模块依赖):
  __strchrnul                                  → see src/string/rust-spec/__strchrnul.md
    // 用于在 s 中查找 '=' 的位置
  unsetenv                                     → see src/env/rust-spec/unsetenv.md
    // 当 s 不含 '=' 时委托处理移除操作
  __environ                                    → see src/env/rust-spec/__environ.md
    // 环境变量数组的全局指针 (AtomicPtr<c_char>)

Predefined Structures/Functions (Rust 核心库 / 语言内置):
  core::ffi::c_char                            // C 字符类型，用于环境字符串指针参数
  core::ffi::c_int                             // C int 类型，用于返回值
  core::sync::atomic::AtomicPtr<EnvRmAddFn>    // 原子函数指针，实现可选依赖的回调注册
  core::sync::atomic::Ordering                 // 内存顺序 (Acquire/Release)，保证环境修改的跨线程可见性
  core::alloc::Global                          // Rust 全局分配器，替代 malloc/realloc/free
  core::ptr::copy_nonoverlapping               // 替代 memcpy
  core::ptr::null_mut                          // 替代 NULL

[GUARANTEE]
Exported Interface (对外导出，必须保持 C ABI 兼容):
  #[no_mangle]
  pub unsafe extern "C" fn putenv(s: *mut core::ffi::c_char) -> core::ffi::c_int;
    // POSIX 标准函数，将 "NAME=VALUE" 格式字符串直接放入进程环境（非拷贝）
    // 前置: s 非 null, 指向 null 结尾的 C 字符串；调用方不得在 putenv 后修改或释放 s
    // 后置: 返回 0 成功; -1 失败 (OOM, 原环境不变)
    // 特殊: 若 s 不含 '=', 委托给 unsetenv(s) 处理

Internal Interface (仅 rusl crate 内部可见，不对外导出):
  pub(crate) unsafe fn putenv_core(s: *mut core::ffi::c_char, l: usize, r: *mut core::ffi::c_char) -> core::ffi::c_int;
    // 环境变量设置核心实现，被 putenv 和 setenv 复用
    // r: 调用方堆分配的字符串指针，putenv 路径传入 null_mut()
  pub(crate) unsafe fn no_op(_old: *mut core::ffi::c_char, _new: *mut core::ffi::c_char);
    // 默认空回调，作为 ENV_RM_ADD 的初始值
  pub(crate) fn register_env_rm_add(f: EnvRmAddFn);
    // 提供内部注册入口，允许 setenv 模块注入内存管理回调
