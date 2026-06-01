# __environ — Rust 接口归约

## 依赖图

```
environ (对外导出全局变量, 定义)
    │
    ├── core::ffi::c_char       (外部依赖 — Rust core 语言项, 不可展开)
    ├── core::ptr::null_mut     (外部依赖 — Rust core 语言项, 不可展开)
    │
    ├──> __libc_start_main      (rusl 内部 — 启动代码, 负责填充 environ)
    └──> __init_tls             (rusl 内部 — TLS 初始化, 备用填充路径)
```

> 本模块为**纯数据定义**，不包含任何函数。无内部函数依赖需要递归追踪。

---

## 原始 C 接口

```c
char **__environ = 0;

weak_alias(__environ, environ);
weak_alias(__environ, ___environ);
weak_alias(__environ, _environ);
```

### C 端符号一览

| 符号名 | 可见性 | 说明 |
|--------|--------|------|
| `__environ` | Internal (不导出) | musl 内部实现名称，实际存储环境变量数组指针的变量 |
| `environ` | **Public** — POSIX 标准 | 用户程序通过 `extern char **environ;` 访问 |
| `___environ` | Internal (不导出) | musl 内部历史别名，与 `__environ` 共享内存 |
| `_environ` | Internal (不导出) | musl 内部历史别名，与 `__environ` 共享内存 |

### weak_alias 机制说明

`weak_alias(old, new)` 宏展开为 `extern __typeof(old) new __attribute__((__weak__, __alias__(#old)))`:
- `__alias__("old")` — `new` 与 `old` 共享同一内存地址
- `__weak__` — 若用户程序定义了同名符号，则用户的定义优先，此处定义被覆盖
- **等价关系**: 四个别名读取/写入任一个等同于操作全部

---

## Rust 设计概要

C 实现中，`__environ` 是内部符号，通过 `weak_alias` 宏将 POSIX 标准名 `environ` 及历史别名 `___environ` / `_environ` 指向同一内存位置。Rust 语言层面不支持 C 风格的 `weak_alias`，因此重新设计如下：

- **仅保留 `environ` 作为唯一的公开导出符号**，使用 `#[no_mangle]` 确保 C ABI 兼容。
- **删除 `__environ`、`___environ`、`_environ` 三个别名**（均为 C 内部实现细节，Rust 实现中无需保留独立符号）。
- **Rust 内部代码直接通过模块私有路径访问 `environ`**，无需通过 `__` 前缀的内部别名。

> **设计理由**: C 的 `__environ` / `_environ` / `___environ` 仅因为 musl 无法直接定义名为 `environ` 的全局变量（该名称在 `<unistd.h>` 中已声明为 `extern`），故需先定义内部名称再通过 `weak_alias` 创建公共别名。Rust 的 `#[no_mangle]` 可直接将 `environ` 定义为公开导出符号，无需中间别名步骤。

> **关于 weak 语义**: 在 `staticlib` 模式下，`.a` 归档中的符号只有在链接器需要解析未定义引用时才会被拉取；若用户程序已定义 `environ`，则不会从归档中提取本模块。这与 C 的 `weak_alias` 在静态链接场景下的行为等价。

---

## Rust 外部 ABI 接口

```rust
// [Visibility]: Public — POSIX.1-2001 标准全局变量。
//               用户程序通过 <unistd.h> 中 extern char **environ; 声明使用。
//               本模块提供该符号的定义。
#[no_mangle]
pub static mut environ: *mut *mut core::ffi::c_char = core::ptr::null_mut::<*mut core::ffi::c_char>();
```

### ABI 兼容性对照

| C 定义 | Rust 定义 | 说明 |
|--------|-----------|------|
| `char **environ;` | `pub static mut environ: *mut *mut c_char` | 类型布局完全兼容 |
| `= 0` (NULL) | `= core::ptr::null_mut()` | 初始值语义等价 |
| 由 `__libc_start_main` 填充 | 由 rusl 启动代码填充 | 初始化流程一致 |

### 别名对照

| C 符号名 | C 可见性 | Rust 状态 |
|----------|----------|-----------|
| `__environ` | Internal (不导出) | **已移除** — Rust 不创建独立符号，内部代码直接访问 `environ` |
| `___environ` | Internal (不导出) | **已移除** — Rust 不创建独立符号 |
| `_environ` | Internal (不导出) | **已移除** — Rust 不创建独立符号 |
| `environ` | **Public** — POSIX 标准 | **保留** — 使用 `#[no_mangle]` 导出为 C ABI 兼容符号 |

---

## 意图 (Intent)

为进程环境变量提供一个全局入口指针。`environ` 指向以空指针终止的字符串指针数组，每个字符串为 `"NAME=VALUE"` 格式的环境变量条目。该指针是所有环境变量操作（`getenv`、`setenv`、`putenv`、`unsetenv`、`clearenv`）的核心数据入口。

### 数据布局

```
environ ──> [0] ──> "HOME=/home/user\0"
            [1] ──> "PATH=/usr/bin\0"
            [2] ──> "LANG=en_US.UTF-8\0"
            ...
            [n] ──> NULL (终止哨兵)
```

---

## 前置条件 (for consumers)

### 读取 environ

```
{P} let e: *mut *mut c_char = environ; {Q}
```

- **前置条件**: `environ` 已被启动代码初始化（`main()` 开始执行后）。在 `main()` 之前（如全局构造函数中），值可能仍为 `null_mut()`。
- **调用者防护**: 调用者应对 `null_mut()` 返回值做防护检查。

### 写入 environ

```
{P} environ = new_array; {Q}
```

- **前置条件**: `new_array` 必须为 `null_mut()` 或指向以 `null_mut()` 终止的 `*mut c_char` 数组。若非 null，数组中每个非 null 字符串必须满足 `"NAME=VALUE"` 格式不变量。
- **内存责任**: 旧环境数组本身**不会被自动释放**。释放旧环境内存是调用者或 `clearenv()` / 相关 API 的责任。

---

## 后置条件 (Postconditions)

### 读取 environ

| 情况 | 结果 |
|------|------|
| 正常情况 (`main()` 已启动) | 返回指向环境变量字符串数组的指针，数组以 `null_mut()` 哨兵终止。调用者可遍历 `*environ.add(i)` 直到值为 `null_mut()` |
| 未初始化 (启动早期阶段) | 返回 `null_mut()` |

### 写入 environ

- `environ` 指向新值。
- 所有通过 `environ` 读取环境变量的操作（如 `getenv()`）立即反映新数组。
- 旧环境数组本身不会被释放。

---

## 不变量 (Invariants)

1. **终止哨兵不变量**: `environ` 若非 `null_mut()`，则指向的字符串数组必须以 `null_mut()` 作为终止标记。即若 `*(environ.add(i))` 为 `null_mut()`，则对于所有 `j > i`，`*(environ.add(j))` 视为越界访问。

2. **格式不变量**: 每个非 null 的条目必须是格式为 `"NAME=VALUE"` 的 NUL 终止字符串，其中：
   - `NAME` 是非空字符串，仅包含可移植字符集（字母、数字、下划线），不能包含 `'='`
   - `VALUE` 可以是任意字符串（可以为空），以 NUL 终止

3. **无重复-NAME 不变量**（POSIX）: 环境变量数组中不应出现同名的条目。若发生，`getenv()` 的行为是返回第一个匹配项（由 rusl 的实现决定）。

4. **所有权不变量**: `environ` 及其中字符串的所有权归属进程。`putenv()` 可能使得其中部分指针指向调用者提供的内存（而非堆分配），此时调用者不得释放该内存直到该条目被覆盖或删除。

5. **进程级共享**: 使用全局静态可变指针，不采用线程局部存储 (TLS)。修改 `environ` 立即影响所有线程的环境变量访问。并发写入是未定义行为（与 POSIX 关于 environ 的线程安全限制一致）。

---

## 系统算法 (System Algorithm)

`__environ` 模块本身为**纯数据定义**，不包含任何函数。其生命周期如下：

```
// 编译期 — 初始化为 null
static mut environ: *mut *mut c_char = core::ptr::null_mut();

// 启动期 — main() 调用之前，由启动代码填充:
//   environ = envp;  // envp 来自内核/CRT 传递的参数

// 运行期 — 由 getenv / setenv / putenv / unsetenv / clearenv 等模块读写
```

### 初始化路径

| 模块 | 角色 | 说明 |
|------|------|------|
| `src/env/__libc_start_main.rs` | 主要初始化者 | 从内核传递的 `envp` 参数获取环境指针，在调用 `main()` 之前赋值给 `environ` |
| `src/env/__init_tls.rs` | 备用初始化路径 | 静态 TLS 场景下设置 `environ` |

---

## 相关文件与使用场景

| 模块 | 关系 | 说明 |
|------|------|------|
| `src/env/getenv.rs` | 使用者 | 遍历 `environ` 数组查找指定 NAME |
| `src/env/putenv.rs` | 使用者 | 修改 `environ` 数组（直接替换指针），不重新分配 |
| `src/env/setenv.rs` | 使用者 | 修改环境时可能重新分配 `environ` 数组 |
| `src/env/unsetenv.rs` | 使用者 | 从 `environ` 数组中移除条目 |
| `src/env/clearenv.rs` | 使用者 | 将 `environ` 置为 `null_mut()` 或空数组 |
| `src/env/__libc_start_main.rs` | 初始化者 | 将 `envp` 参数赋值给 `environ`，完成初始化 |
| `src/env/__init_tls.rs` | 初始化者 | TLS 初始化流程中设置 `environ` |

---

## 内部实现设计要点

- `environ` 为 `pub static mut`，因 Rust 编译器对 `static mut` 不做别名分析优化，所有访问需通过 `unsafe` 块进行裸指针操作。
- 内部实现的 `unsafe` 范围应限制在必要的指针读写操作上，不将大段逻辑包裹在 `unsafe` 中。
- 由于 `environ` 不使用原子类型（与 musl 保持一致的简单全局指针语义），多线程并发写入是未定义行为。读取操作本身是安全的（读指针值），但若其他线程同时调用 `putenv`/`setenv`/`unsetenv`，行为未定义。
- 若未来需要对 `environ` 的读写提供更强的线程安全保证，可考虑将类型提升为 `AtomicPtr<*mut c_char>`。当前设计优先保持与 musl 原始语义的一致性和零开销。

---

```
/* Rely */
[RELY]
Predefined Structures/Functions:
  core::ffi::c_char                              // 依赖1: C 字符类型，用于环境字符串指针的元素类型
  core::ptr::null_mut                            // 依赖2: null 指针常量，用于初始值和终止哨兵

Predefined Macros/Traits:
  (none)

[GUARANTEE]
Exported Interface:
  // 全局变量: environ (POSIX 标准对外导出)
  //   ABI 兼容 C 的 extern char **environ;
  //   环境变量数组的指针，初值为 null_mut()，由启动代码在 main() 前填充。
  //   数组以 null_mut() 终止，每个元素为 "NAME=VALUE" 格式的 NUL 终止 C 字符串。
  #[no_mangle]
  pub static mut environ: *mut *mut core::ffi::c_char;

Internal Interface:
  // 本模块仅包含一个对外导出的全局变量定义，无内部辅助函数。
  // C 原版中的 __environ / ___environ / _environ 别名不再保留为独立符号。
  (none)
```
