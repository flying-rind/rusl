# glob / globfree Rust 接口规约

## 概述

本模块实现 POSIX `glob()` 和 `globfree()` 函数。`glob()` 根据 shell 通配符规则查找文件系统中匹配指定模式的路径名，支持通配字符（`*`、`?`、`[...]`）、波浪号展开（`~`）、目录标记等功能。Rust 实现中，内部 `struct match` 链表、递归引擎 `do_glob` 等可用 Rust 安全抽象重构，但对外 `glob` / `globfree` 签名必须保持 ABI 兼容。

---

## 依赖图

```
glob (Public)
  ├── expand_tilde (Internal) — 波浪号展开
  ├── do_glob (Internal)       — 核心递归引擎
  ├── append (Internal)        — 结果链表追加
  ├── freelist (Internal)      — 释放结果链表
  ├── ignore_err (Internal)    — 默认错误处理
  └── sort (Internal)          — qsort 回调

globfree (Public)
  └── 直接释放 glob_t 内部分配的内存

外部依赖（不在此生成规约）:
  - fnmatch (src/regex/fnmatch.c)  — 路径组件匹配
  - stat / lstat / opendir / readdir / closedir (libc) — 文件系统操作
  - malloc / free / realloc (libc) — 内存分配
  - getenv / getpwnam_r / getpwuid_r / getuid (libc)  — 用户信息
  - strcmp / qsort (libc)         — 字符串比较/排序
  - __strchrnul (src/string)      — 字符查找
```

---

## [RELY]

Predefined Structures/Functions:
  `glob_t` (struct, `<glob.h>`)                                // 依赖1: POSIX glob 结果类型
  `fnmatch` (fn, regex/fnmatch 模块)                            // 依赖2: 通配符匹配
  `__strchrnul` (fn, string/strchrnul 模块)                     // 依赖3: 字符查找（仅 expand_tilde 使用）
  `stat` / `lstat` / `opendir` / `readdir` / `closedir` (libc) // 依赖4: 文件系统操作
  `malloc` / `free` / `realloc` (libc)                         // 依赖5: 内存管理
  `getenv` / `getpwnam_r` / `getpwuid_r` / `getuid` (libc)     // 依赖6: 用户/环境信息
  `strcmp` / `qsort` (libc)                                     // 依赖7: 字符串操作
  `c_char`, `c_int`, `c_size_t` (std::ffi / libc 类型)          // 依赖8: C ABI 兼容类型
  `PATH_MAX` (system constant)                                  // 依赖9: 路径最大长度

---

## [GUARANTEE]

Exported Interface:

```rust
extern "C" fn glob(pat: *const c_char, flags: c_int, errfunc: Option<unsafe extern "C" fn(*const c_char, c_int) -> c_int>, g: *mut glob_t) -> c_int
extern "C" fn globfree(g: *mut glob_t)
```

本模块保证对外提供的接口签名，ABI 兼容 POSIX `glob()` / `globfree()`。

---

## 内部类型定义

### MatchNode — 匹配结果节点

C 实现使用 `struct match { struct match *next; char name[]; }` 灵活数组，并通过 `offsetof` 从 `name` 指针反推分配块起始地址。Rust 实现可用更安全的方式：

```rust
// [Visibility]: Internal — rusl crate 内部
pub(crate) struct MatchNode {
    name: CString,      // 文件名（含可能的 '/' 后缀用于 GLOB_MARK）
    next: Option<Box<MatchNode>>,  // 链表后继
}
```

**Rust 设计优势**：
- 使用 `CString` 替代灵活数组成员，由 `CString` 管理 null 终止符
- 使用 `Option<Box<MatchNode>>` 替代裸指针链表，通过 `Box` 的 RAII 语义自动管理内存
- 不再需要 `offsetof` 反推技巧；`globfree` 中通过遍历链表并 drop 来释放
- 哑头节点可用 `Option<Box<MatchNode>>` 的 `None` 或独立的哑 `MatchNode` 表示

**不变量**：
- 链表头节点为哑节点（`name` 为空），`next` 指向第一个实际匹配项
- 每个节点的 `name` 以 null 终止
- 若 `GLOB_MARK` 设置且目标为目录，`name` 末尾附加 `'/'`

---

## 内部辅助函数

### append — 追加匹配结果

```rust
// [Visibility]: Internal — rusl crate 内部
pub(crate) fn append(tail: &mut Option<Box<MatchNode>>, name: &CStr, mark: bool) -> Result<(), GlobError>
```

**意图**：向 match 链表尾部追加一个新的匹配项节点。若 `mark` 为真且文件名不以 `'/'` 结尾，则附加 `'/'` 后缀。

**前置条件**：
- `tail` 指向链表当前尾部位置
- `name` 为有效的 null 终止字符串

**后置条件**：

| 条件 | 返回值 | 状态变化 |
|------|--------|----------|
| 分配成功 | `Ok(())` | 新节点追加到链表尾部，`tail` 更新 |
| 分配失败 (OOM) | `Err(GlobError::NoSpace)` | 链表不变，`errno = ENOMEM` |

---

### ignore_err — 默认错误处理

```rust
// [Visibility]: Internal — rusl crate 内部
pub(crate) extern "C" fn ignore_err(_path: *const c_char, _err: c_int) -> c_int { 0 }
```

**意图**：当用户未提供自定义 `errfunc` 时，作为 `glob` 的默认错误处理回退。该函数忽略所有错误，始终返回 0。

**前置条件**：无特别要求。

**后置条件**：始终返回 `0`（非零返回值会触发 `glob` 中止）。

---

### sort — 比较回调

```rust
// [Visibility]: Internal — rusl crate 内部
pub(crate) fn sort_cmp(a: &&*const c_char, b: &&*const c_char) -> Ordering
```

**意图**：对 `gl_pathv` 指针数组进行字典序排序。Rust 实现中使用标准库的 `sort_unstable_by` 替代 C 的 `qsort`。

---

### freelist — 释放结果链表

```rust
// [Visibility]: Internal — rusl crate 内部
// Rust 实现中，通过 drop MatchNode 链表的 Box 层次自动释放，
// 无需独立函数。若需显式清理，可提供一个消费函数。
pub(crate) fn freelist(head: Option<Box<MatchNode>>) {
    // head 被 drop 时，递归释放整个链表
}
```

---

### expand_tilde — 波浪号展开

```rust
// [Visibility]: Internal — rusl crate 内部
pub(crate) fn expand_tilde(
    pat: &CStr,
    buf: &mut [u8],
    pos: &mut usize,
) -> Result<(), GlobError>
```

**意图**：将 glob 模式开头的 `~` 或 `~username/` 展开为对应的用户家目录路径。

**系统算法**：
1. 解析 `~` 后的用户名（到 `/` 或 `\0` 为止）
2. 若用户名为空：通过 `HOME` 环境变量获取；若 `HOME` 为空，回退到 `getpwuid_r(getuid(), ...)`
3. 若用户名非空：通过 `getpwnam_r` 查询
4. 将家目录路径写入 `buf`，追加原始分隔符 `/`（若存在）
5. 更新 `*pos` 和推进 `pat` 指针

**前置条件**：
- `pat` 指向以 `~` 字符开头的模式字符串
- `buf` 长度为 `PATH_MAX`
- `flags` 包含 `GLOB_TILDE` 或 `GLOB_TILDE_CHECK`

**后置条件**：

| 条件 | 返回值 | 状态变化 |
|------|--------|----------|
| 家目录展开成功 | `Ok(())` | `buf[0..*pos]` 包含家目录路径 |
| 内存不足 | `Err(GlobError::NoSpace)` | `buf` 内容未定义 |
| 用户不存在 | `Err(GlobError::NoMatch)` | `buf` 内容未定义 |

---

### do_glob — 核心递归引擎

```rust
// [Visibility]: Internal — rusl crate 内部
pub(crate) fn do_glob(
    buf: &mut [u8],
    pos: &mut usize,
    type_hint: DirentType,
    pat: &CStr,
    flags: GlobFlags,
    errfunc: Option<unsafe extern "C" fn(*const c_char, c_int) -> c_int>,
    tail: &mut Option<Box<MatchNode>>,
) -> Result<(), GlobError>
```

**意图**：`glob` 的核心递归实现。逐级解析路径模式，将通配符匹配委托给 `fnmatch`，通过目录遍历枚举候选文件。

**系统算法**（逐级路径递归）：

1. **类型快速修正**：若调用者未传入 `type_hint` 且未要求 `GLOB_MARK`，将类型设为 `DT_REG` 以避免不必要的 `stat` 调用。

2. **全是斜杠的边界处理**：若剩余模式全由 `/` 组成且当前类型不是目录，清空类型以触发存在性检查。

3. **模式前缀逐字扫描**：
   - 逐字节扫描 `pat`，遇到 `*`、`?` 或括号外部的 `]` 时停止
   - 转义处理：`\\` 跳过反斜杠并取下一字符
   - 遇到 `/` 时：重置括号状态，截断当前路径组件并写入 `buf`
   - 溢出处理：若 `pos + (j+1) >= PATH_MAX` 且当前在括号内部，设置溢出标记

4. **路径组件消费完成后的处理**：
   - 若模式耗尽：执行终点处理（`GLOB_MARK`、`stat`/`lstat`、调用 `append`）
   - 若含通配符：执行目录遍历

5. **目录遍历**：
   - 在剩余模式中定位 `/`，区分字面 `/` 与被转义的 `/`
   - `opendir` 打开当前累积路径所在目录
   - 遍历 `readdir` 返回的每个目录项：
     - 跳过名称过长的条目
     - 使用 `fnmatch` 测试文件名是否匹配当前模式组件
     - 若匹配成功，递归调用 `do_glob` 处理下一级组件

**前置条件**：
- `buf` 指向长度为 `PATH_MAX` 的数组；`buf[0..pos-1]` 包含当前累积的路径前缀
- `pos < PATH_MAX`
- `pat` 指向待匹配的剩余模式（应当已经过 `expand_tilde` 处理）

**后置条件**：

| 条件 | 返回值 | 状态变化 |
|------|--------|----------|
| 正常完成 | `Ok(())` | 匹配结果通过 `append` 写入链表 |
| 内存分配失败 | `Err(GlobError::NoSpace)` | `closedir` 确保目录描述符不泄漏 |
| 目录操作错误且需中止 | `Err(GlobError::Aborted)` | `closedir` 已调用，错误已报告 |

**不变量**：
- `pos` 始终等于 `buf` 中累积路径的长度（不含 null 终止符）
- `buf[pos]` 在调用入口处为 null 终止符，递归返回后仍为 null 终止符
- 每次 `opendir` 成功必有一次对应的 `closedir`，不存在目录描述符泄漏
- 递归深度受路径深度自然限制（每个路径组件调用一次递归）

---

## glob (对外导出)

```rust
#[no_mangle]
pub unsafe extern "C" fn glob(
    pat: *const c_char,
    flags: c_int,
    errfunc: Option<unsafe extern "C" fn(*const c_char, c_int) -> c_int>,
    g: *mut glob_t,
) -> c_int
```

[Visibility]: Public — POSIX.1-2001 标准函数，`<glob.h>` 声明。

### 意图 (Intent)

根据 shell 使用的规则，查找文件系统中匹配指定模式 `pat` 的路径名。结果存储于 `glob_t` 结构中。

### 系统算法 (System Algorithm)

```
1. 初始化哑头节点，设置 errfunc 默认值为 ignore_err
2. 非追加模式重置: gl_pathc = 0, gl_pathv = NULL
3. 拷贝模式字符串 (strdup)
4. 若 GLOB_TILDE 且模式以 ~ 开头: expand_tilde 展开
5. 调用 do_glob 执行核心匹配
6. 释放模式拷贝
7. 若错误: 释放链表，返回错误码
8. 计数匹配数量
9. 若无匹配且 GLOB_NOCHECK: 将原模式作为唯一匹配
10. 若无匹配且未出错: 返回 GLOB_NOMATCH
11. 分配结果数组 (malloc / realloc 若 GLOB_APPEND)
12. 遍历链表填充 gl_pathv
13. 若未设 GLOB_NOSORT: qsort 排序
14. 返回 0 或首个错误码
```

### 前置条件

- `pat` 为以 null 终止的合法 glob 模式字符串（不能为 NULL）
- `g` 为非 NULL 的 `glob_t` 指针
- 若 `flags & GLOB_APPEND`，`g` 应为先前有效 `glob()` 调用的结果
- `flags` 为 `GLOB_*` 标志的有效按位或组合

### 后置条件

| 条件 | 返回值 | 状态变化 |
|------|--------|----------|
| 匹配成功 | `0` | `g->gl_pathc` 为匹配数；`g->gl_pathv` 包含匹配路径 |
| 无匹配且无 `GLOB_NOCHECK` | `GLOB_NOMATCH` | `gl_pathc = 0` |
| 无匹配但 `GLOB_NOCHECK` | `0` | `gl_pathv[gl_offs]` 指向原 `pat` 副本 |
| 内存不足 | `GLOB_NOSPACE` | 已分配内存已释放，`g` 保持调用前状态 |
| 目录访问错误且需中止 | `GLOB_ABORTED` | 可能包含部分结果 |

### 资源管理契约

- 调用者必须通过 `globfree(g)` 释放成功调用所分配的资源
- 返回 `GLOB_NOSPACE` 时，`glob` 内部已释放所有临时分配

---

## globfree (对外导出)

```rust
#[no_mangle]
pub unsafe extern "C" fn globfree(g: *mut glob_t)
```

[Visibility]: Public — POSIX.1-2001 标准函数，`<glob.h>` 声明。

### 意图 (Intent)

释放由先前 `glob()` 调用在 `glob_t` 结构中分配的所有内存。

在 Rust 内部实现中，由于 `MatchNode` 链表使用 `Box` 管理，`globfree` 的核心逻辑是：
1. 遍历 `gl_pathv` 数组，通过指针反推 `MatchNode` 分配块并 drop
2. 释放 `gl_pathv` 数组本身
3. 重置 `gl_pathc = 0`，`gl_pathv = NULL`

注意：由于 ABI 兼容要求，`gl_pathv` 数组中存储的必须是直接指向 `MatchNode.name` 的 `char *` 指针，以便通过 `offsetof` 反推原始分配块。Rust 内部实现需要保持此内存布局约定。

### 前置条件

- `g` 非 NULL，且其内容由先前的 `glob()` 调用产生（或已初始化为全零/`globfree` 重置后）
- 多次调用 `globfree` 对同一 `g` 是安全的

### 后置条件

- 所有 `glob()` 为 `g` 分配的内存均已释放
- `g->gl_pathc == 0`，`g->gl_pathv == NULL`
- 无返回值。函数不会失败。

---

## 引用关系

| 符号 | 可见性 | 被引用者 |
|------|--------|----------|
| `glob` | Public | `<glob.h>` |
| `globfree` | Public | `<glob.h>` |
| `MatchNode` | Internal (`pub(crate)`) | `append`, `do_glob`, `freelist` |
| `do_glob` | Internal (`pub(crate)`) | 仅 `glob` 调用（含自身递归） |
| `expand_tilde` | Internal (`pub(crate)`) | 仅 `glob` 调用 |
| `append` | Internal (`pub(crate)`) | `do_glob` 调用 |
| `ignore_err` | Internal (`pub(crate)`) | `glob` 默认 `errfunc` |
| `sort_cmp` | Internal (`pub(crate)`) | `glob` 排序时使用 |
| `freelist` | Internal (`pub(crate)`) | `glob` 清理时调用 |
