# setenv.rs 规约

## 原始 C 接口
```c
int setenv(const char *var, const char *value, int overwrite);
void __env_rm_add(char *old, char *new);
```

---

## 依赖图

```
setenv (Public, extern "C")
  ├── __strchrnul(const char *, int)       → src/string/ (跨模块内部函数)
  ├── getenv(const char *)                 → src/env/getenv.rs (跨模块，POSIX 公开接口)
  ├── strlen(const char *)                 → 在 no_std 下由内部实现替代
  ├── alloc::alloc::alloc                  → 内部全局分配器 (malloc 的 Rust 替代)
  ├── core::ptr::copy_nonoverlapping       → core (替代 C memcpy)
  ├── __putenv(char *, size_t, char *)     → src/env/putenv.rs (跨模块内部函数)
  │     ├── alloc::alloc + realloc + dealloc (全局分配器)
  │     ├── __strchrnul (跨模块内部)
  │     ├── __environ (跨模块全局变量)
  │     └── __env_rm_add → 跨模块 static mut 函数指针 Hook
  │           └── 本模块注册 env_rm_add_impl (module-private)
  ├── __environ                             → src/env/__environ.rs (跨模块全局变量)
  └── errno / EINVAL                       → 内部 errno 模块

env_rm_add_impl (Internal, module-private)
  ├── alloc::alloc::dealloc                → 全局分配器 (free 的 Rust 替代)
  ├── alloc::alloc::realloc                → 全局分配器 (realloc 的 Rust 替代)
  └── [static] ENV_ALLOCED, ENV_ALLOCED_N  → 本模块私有静态变量
```

> **关键架构说明**: C 实现中 `__env_rm_add` 在 `setenv.c` 中为强符号定义（strong definition），通过 ELF 链接器弱符号覆盖机制替换 `putenv.c`、`unsetenv.c`、`clearenv.c` 中的 `dummy` 备选。Rust 实现将此改为 `static mut` 函数指针注册机制（定义于 `putenv` 模块）:
> ```rust
> // putenv 模块中定义
> #[allow(static_mut_refs)]
> static mut __env_rm_add: fn(old: *mut c_char, new: *mut c_char) = dummy;
> ```
> `setenv` 模块在首次被调用时将此函数指针设置为模块私有的 `env_rm_add_impl`。只有 `setenv` 被实际使用时，内存追踪逻辑才激活；未使用 `setenv` 的程序不承担额外的内存管理开销。这与 C 弱符号机制的语义完全等价，但避免了对 ELF 特有链接器行为的依赖。

---

## env_rm_add_impl（内部内存管理函数）

```rust
unsafe fn env_rm_add_impl(old: *mut c_char, new: *mut c_char);
```

**[Visibility]: Internal** — 模块私有 (`pub(crate)` 或更小可见性)，不对外导出。该函数被注册到 `putenv` 模块的 `__env_rm_add` Hook 函数指针上，负责追踪和释放由 `setenv` 在堆上分配的环境变量字符串。声明于 putenv 模块的 Hook 为 `fn(*mut c_char, *mut c_char)` 签名，本函数与之匹配。

### 涉及的静态状态

```rust
// 动态数组，存储指向堆分配环境变量字符串的原始指针
// 使用原始指针数组 + 长度以最小化运行时开销，同时与 no_std 环境完全兼容
static mut ENV_ALLOCED: *mut *mut c_char = core::ptr::null_mut();
static mut ENV_ALLOCED_N: usize = 0;
```

**不变量 (Invariant)**:
1. `ENV_ALLOCED` 中非 NULL 条目均为指向堆上 "NAME=VALUE" 格式字符串的有效指针。
2. `ENV_ALLOCED` 中可能包含 NULL 条目（表示已被释放但槽位待复用的位置）。
3. `ENV_ALLOCED_N` 等于 `ENV_ALLOCED` 数组的逻辑长度（即分配的元素数量）。
4. `ENV_ALLOCED` 仅由 `env_rm_add_impl` 修改，其他模块仅间接通过 `__putenv` 传参触发。
5. 同一条 env 字符串指针在 `ENV_ALLOCED` 中至多出现一次（无重复追踪）。

### 前置条件 (Precondition)

- 无特殊前提条件。`old` 和 `new` 可为任意指针（包括 `null_mut()`）。
- 调用者（`__putenv`、`unsetenv`、`clearenv`）保证：
  - 当 `old` 非 NULL 时，它指向一个之前已通过 `env_rm_add_impl` 追踪的环境变量字符串，且调用者正在从 `__environ` 中移除或替换该条目。
  - 当 `new` 非 NULL 时，它指向一个堆上新分配的 "NAME=VALUE" 格式字符串，需要被追踪以便未来释放。

### 后置条件 (Postcondition)

分四种典型调用模式描述：

**Case 1 — 替换环境变量（`old != null`，`new != null`）**:
- 在 `ENV_ALLOCED` 中线性扫描查找匹配 `old` 的条目。
- 若找到：用 `new` 替换该条目，调用 `alloc::dealloc(old)` 释放旧字符串。
- 若 `old` 不在 `ENV_ALLOCED` 中（如源自父进程的原始 environ）：将 `new` 追加到 `ENV_ALLOCED`（复用 NULL 槽或 realloc 扩容）。
- 若扩容失败（`realloc` 返回 null）：静默丢弃 `new` 的追踪，但旧字符串正常释放。

**Case 2 — 删除环境变量（`old != null`，`new == null`）**:
- 在 `ENV_ALLOCED` 中线性扫描查找匹配 `old` 的条目。
- 若找到：将该条目标记为 null（释放槽位），调用 `alloc::dealloc(old)` 释放旧字符串。
- 若未找到：无操作（`old` 来自父进程 environ，由父进程管理生命周期）。

**Case 3 — 添加新环境变量（`old == null`，`new != null`）**:
- 首先尝试在 `ENV_ALLOCED` 中寻找已有的 NULL 槽位复用。
- 若找到 NULL 槽位：将 `new` 填入该槽位。
- 若未找到：调用 `realloc` 扩容（容量 +1），将 `new` 追加到末尾，`ENV_ALLOCED_N++`。
- 若 `realloc` 失败：静默返回，`new` 不被追踪（内存泄漏，但为有限泄漏且进程通常即将终止）。

**Case 4 — NOP（`old == null`，`new == null`）**:
- 无任何操作，立即返回。

### 系统算法 (System Algorithm)

使用单次线性扫描 + 复用空闲槽的策略：

```
算法 env_rm_add_impl:
  输入: old: *mut c_char, new: *mut c_char
  输出: 无

  for i in 0..ENV_ALLOCED_N:
    若 ENV_ALLOCED[i] == old:                  // 找到要替换的条目
        ENV_ALLOCED[i] = new
        alloc::dealloc(old 转换为 *mut u8, ...)  // 释放旧字符串
        return
    若不满足 且 ENV_ALLOCED[i] == null 且 new != null:  // 找到空闲槽位
        ENV_ALLOCED[i] = new                     // 填入新值
        new = null_mut()                         // 标记已放置
  // 循环结束
  若 new == null: return                         // 已放置或无需操作
  // 否则需要扩容
  t = alloc::realloc(ENV_ALLOCED as *mut u8,
                     Layout::array::<*mut c_char>(ENV_ALLOCED_N + 1).unwrap(),
                     ...)
  若 t == null: return                           // 分配失败，静默放弃追踪
  ENV_ALLOCED = t as *mut *mut c_char
  ENV_ALLOCED[ENV_ALLOCED_N] = new
  ENV_ALLOCED_N += 1
```

**关键设计决策**:
1. **单次扫描、双条件检查**: 在同一循环中同时检查"替换"和"放置"条件。这避免了两次独立的扫描，但引入一个微妙语义：当 `new` 在 NULL 槽被放置（`new = null_mut()`）后，若后续迭代匹配到 `old`，`ENV_ALLOCED[i] = null_mut()` 会将 old 的槽位清零（而非常规替换），此行为对调用者透明，无副作用。
2. **NULL 槽复用**: 未被 `realloc` 回收的槽位保持为 null，等待后续添加操作复用。这避免了频繁的 realloc 调用。
3. **静默 realloc 失败**: 不在 realloc 失败时返回错误，因为此时系统已接近 OOM，且丢失对单个环境字符串的追踪不影响程序继续运行（字符串本身已在 `__environ` 中可用）。

### 线程安全

本函数**不是**线程安全的。它操作模块私有静态变量 `ENV_ALLOCED` 和 `ENV_ALLOCED_N` 而无任何同步机制。POSIX 标准明确规定 `setenv`、`unsetenv`、`putenv` 等函数不是线程安全的，故调用者负责外部同步，本函数无需内部加锁。

---

## setenv（对外导出）

```rust
extern "C" fn setenv(
    var: *const core::ffi::c_char,
    value: *const core::ffi::c_char,
    overwrite: core::ffi::c_int,
) -> core::ffi::c_int;
```

**[Visibility]: External** — POSIX.1-2001 标准函数，声明于 `<stdlib.h>`。用户程序可直接调用。必须保持与 C ABI 完全兼容。

### 意图 (Intent)

向进程环境变量列表中添加或更新一个环境变量。该函数构造 "NAME=VALUE" 格式的字符串并将其插入到环境变量数组中。与 `putenv` 不同，`setenv` 会自行分配并复制字符串，而非要求调用者管理内存。

### 前置条件 (Precondition)

1. `var` 必须为指向以 NUL 结尾的非空 C 字符串的有效指针（`!var.is_null()`）。
2. `var` 的内容必须满足以下所有条件：
   - 非空字符串（长度 > 0）。
   - 不包含 `=` 字符。
3. `value` 必须为指向以 NUL 结尾的 C 字符串的有效指针（允许空字符串 `""`）。
4. `overwrite` 取值为 `0` 或非 0 整数。
5. 调用者负责确保线程安全（POSIX 未规定 `setenv` 为线程安全函数）。

### 后置条件 (Postcondition)

**Case 1 — 参数校验失败**:
- 条件: `var.is_null()`，或 `var` 指向空字符串（长度 0），或 `var` 中包含 `=` 字符。
- 行为: 设置 `errno = EINVAL`，返回 `-1`。
- 环境变量列表不发生任何变化。

**Case 2 — 变量已存在且 `overwrite == 0`**:
- 条件: 校验通过，`getenv(var)` 返回非 NULL，且 `overwrite == 0`。
- 行为: 返回 `0`，环境变量列表不发生任何变化。
- 不分配新内存。

**Case 3 — 内存分配失败**:
- 条件: 校验通过，且（变量不存在 或 `overwrite != 0`），但 `alloc::alloc::alloc` 返回 null。
- 行为: 返回 `-1`。`errno` 的值取决于分配器实现（通常为 `ENOMEM`）。
- 环境变量列表不发生任何变化。

**Case 4 — 成功添加/更新**:
- 条件: 校验通过，且（变量不存在 或 `overwrite != 0`），且内存分配成功。
- 行为:
  1. 分配大小为 `strlen(var) + strlen(value) + 2` 字节的堆内存。
  2. 在此内存中构造字符串 `"var=value"`（格式: `var` + `=` + `value` + `\0`）。
  3. 调用 `__putenv(s, l1, s)` 将构造的字符串插入环境变量数组:
     - 若 `var` 已存在对应条目：替换旧条目，通过 `__env_rm_add(old, s)` 释放旧字符串并注册新字符串。
     - 若 `var` 不存在：在 `__environ` 数组中追加新条目，通过 `__env_rm_add(null, s)` 注册新分配的字符串。
  4. 返回 `0`（`__putenv` 的返回值，成功为 0）。
- 副作用:
  - `__environ` 数组可能被重新分配（扩容）。
  - `__environ` 中特定条目的指针可能被修改。
  - `ENV_ALLOCED`（模块私有的追踪数组）可能被更新。

### 系统算法 (System Algorithm)

```
算法 setenv:
  输入: var: *const c_char, value: *const c_char, overwrite: c_int
  输出: 0 (成功) 或 -1 (失败)

  // 1. 参数校验
  若 var.is_null(): goto invalid
  eq_pos = __strchrnul(var, b'=' as c_int)       // 查找 '=' 或末尾 NUL
  l1 = eq_pos.offset_from(var) as usize          // 计算 var 的长度
  若 l1 == 0: goto invalid                       // 空字符串
  若 *eq_pos != 0: goto invalid                  // 找到了 '='，即 var 中含 '='

  // 2. 惰性初始化：确保 env_rm_add_impl 已注册到 __env_rm_add Hook
  ensure_env_init()                               // 将 putenv::__env_rm_add 设置为 env_rm_add_impl

  // 3. 检查 overwrite 策略
  若 overwrite == 0 且 getenv(var) 非 null:
      返回 0

  // 4. 构造新字符串
  l2 = strlen(value)
  layout = Layout::from_size_align(l1 + l2 + 2, 1).unwrap()
  s = alloc::alloc::alloc(layout)                // +2: '=' + '\0'
  若 s.is_null(): 返回 -1
  // 复制 var + '=' + value + '\0'
  core::ptr::copy_nonoverlapping(var as *const u8, s, l1)
  *s.add(l1) = b'='
  core::ptr::copy_nonoverlapping(value as *const u8, s.add(l1 + 1), l2 + 1)

  // 5. 插入环境
  返回 __putenv(s as *mut c_char, l1, s as *mut c_char)

invalid:
  errno = EINVAL
  返回 -1
```

### 参数语义说明

- **`var`**: 环境变量名。不能为空，不能含 `=`。因为 `=` 是环境变量"名称=值"格式的分隔符。
- **`value`**: 环境变量值。可以为空字符串 `""`，此时环境变量被设置为空值（如 `"PATH="`）。
- **`overwrite`**: 覆盖标志。
  - `0`: 若变量已存在，不修改其值，返回成功。这允许"默认值"语义。
  - 非 0: 无论变量是否存在，始终更新其值。

### POSIX 标准兼容性

本实现符合 POSIX.1-2001 规范：

| POSIX 要求 | musl 实现 | rusl 实现 |
|---|---|---|
| `var` 含 `=` 时返回 -1，设置 `errno = EINVAL` | 符合 | 符合 |
| `var` 为空字符串的行为 | POSIX 未规定，musl 返回 -1 + EINVAL | 符合 musl 行为 |
| 成功返回 0 | 符合 | 符合 |
| 失败返回 -1 | 符合 | 符合 |
| 分配并复制字符串 | 符合（`malloc` + `memcpy`） | 符合（`alloc` + `copy_nonoverlapping`） |

### 与 putenv 的对比

| 特性 | setenv | putenv |
|---|---|---|
| 字符串所有权 | rusl 内部分配并拥有（加入 `ENV_ALLOCED` 追踪） | 调用者拥有（仅存储指针，不复制） |
| 自动释放 | 是（通过 `env_rm_add_impl` 追踪） | 否（调用者负责生命周期） |
| 接口复杂度 | 更高（需要分别传 name 和 value） | 更简单（传 "NAME=VALUE" 字符串） |
| 内存安全性 | 更安全（内部管理） | 危险（调用者修改字符串会影响环境） |

### 跨模块依赖说明

1. **`__strchrnul`** (`src/string/__strchrnul.rs`): 内部字符串查找函数。返回指向 `s` 中字符 `c` 首次出现位置的指针，若未找到则返回指向末尾 NUL 的指针。用于高效计算 `var` 长度并同时检测是否含 `=`。

2. **`__putenv`** (`src/env/putenv.rs`): 内部环境变量插入核心函数。负责在 `__environ` 数组中查找/替换/追加条目，管理 `__environ` 的重新分配，以及调用 `__env_rm_add` 追踪堆分配字符串的生命周期。详见 `src/env/rust-spec/putenv.md`。

3. **`getenv`** (`src/env/getenv.rs`): POSIX 标准函数。在 `__environ` 中按名称查找环境变量并返回值字符串指针。用于 `overwrite == 0` 时检查变量是否已存在。

4. **`__env_rm_add`** (`src/env/putenv.rs` 中定义): 跨模块 `static mut` 函数指针 Hook，签名 `fn(*mut c_char, *mut c_char)`，默认指向 `dummy` 空实现。`setenv` 模块在首次调用时通过 `ensure_env_init` 将其设置为模块私有的 `env_rm_add_impl`，负责追踪和释放堆分配的环境字符串。

5. **`__environ`** (`src/env/__environ.rs`): 全局变量，指向进程环境变量指针数组（NULL 终止）。也通过 `weak_alias` 导出为 `environ`。

---

## Rust 与 C 实现差异总览

| 项目 | C 实现 | Rust 实现 |
|------|--------|-----------|
| `__env_rm_add` 定义方式 | 强符号定义 + ELF 弱符号覆盖 | `putenv` 模块 `static mut` 函数指针，本模块注册 `env_rm_add_impl` |
| 追踪数组 `env_alloced` | `static char **env_alloced` | `static mut ENV_ALLOCED: *mut *mut c_char`（模块私有） |
| 追踪数组长度 | `static size_t env_alloced_n` | `static mut ENV_ALLOCED_N: usize`（模块私有） |
| `malloc` | libc `malloc(l1+l2+2)` | `alloc::alloc::alloc(Layout)` |
| `memcpy` | libc `memcpy` | `core::ptr::copy_nonoverlapping` |
| `strlen` | libc `strlen` | `CStr::to_bytes().len()` 或内部实现 |
| `__strchrnul` | C 跨模块调用 | Rust 跨模块调用（同接口） |
| `getenv` | C 跨模块调用 | Rust 跨模块调用（同接口，`extern "C"` ABI） |
| `__putenv` | C 跨模块调用 | Rust 跨模块调用（同接口，`pub(crate)`） |
| 初始化时机 | 链接时弱符号解析 | 首次 `setenv` 调用时惰性注册 |
| `errno / EINVAL` | libc | 内部 errno 模块 |
| 禁止使用 std | N/A | 全程使用 `core` + `alloc`（no_std 兼容） |

---

/* Rely */
[RELY]
Predefined Structures/Functions:
  core::ffi::c_char                          // 依赖1: C 字符类型，用于环境字符串
  core::ffi::c_int                           // 依赖2: C int 类型，用于返回值
  core::ptr::copy_nonoverlapping             // 依赖3: 内存拷贝，替代 memcpy
  core::ptr::null_mut                        // 依赖4: 空指针，用于指针比较和赋值
  core::alloc::Layout                        // 依赖5: 内存布局描述，用于 alloc/realloc/dealloc
  alloc::alloc::alloc(layout: Layout) -> *mut u8
                                             // 依赖6: 堆内存分配，替代 malloc
  alloc::alloc::dealloc(ptr: *mut u8, layout: Layout)
                                             // 依赖7: 堆内存释放，替代 free
  alloc::alloc::realloc(ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8
                                             // 依赖8: 堆内存重新分配，替代 realloc
  __strchrnul(s: *const c_char, c: c_int) -> *const c_char
                                             // 依赖9: 跨模块内部函数，查找字符位置
  getenv(name: *const c_char) -> *mut c_char // 依赖10: 跨模块 POSIX 公开函数，按名称查找环境变量
  __putenv(s: *mut c_char, l: usize, r: *mut c_char) -> c_int
                                             // 依赖11: 跨模块内部函数，核心环境变量插入逻辑
  __environ: *mut *mut c_char               // 依赖12: 跨模块全局变量，环境变量指针数组
  putenv::__env_rm_add: static mut fn(*mut c_char, *mut c_char)
                                             // 依赖13: 跨模块 Hook 函数指针，定义于 putenv 模块
                                             //         setenv 在首次调用时将其设置为 env_rm_add_impl
  errno::EINVAL                              // 依赖14: EINVAL 常量，用于参数校验失败

Predefined Macros/Traits:
  (none)

[GUARANTEE]
Exported Interface:
  extern "C" fn setenv(
      var: *const core::ffi::c_char,
      value: *const core::ffi::c_char,
      overwrite: core::ffi::c_int,
  ) -> core::ffi::c_int;
                                             // 本模块保证对外提供的 ABI 兼容接口

Internal Interface:
  pub(crate) unsafe fn env_rm_add_impl(
      old: *mut core::ffi::c_char,
      new: *mut core::ffi::c_char,
  );
                                             // 内部内存追踪函数，注册到 putenv::__env_rm_add Hook
                                             // 签名与 putenv 模块的 __env_rm_add 函数指针类型完全匹配
                                             // 负责接收/释放/追踪堆分配的环境变量字符串
