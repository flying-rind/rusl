# tempnam 函数规约

## 复杂度分级: Level 2

> musl libc 标准库可定制临时文件名生成函数（已过时）。允许指定目录和前缀生成唯一文件名，并动态分配返回缓冲区。Rust 实现中，外部接口保持 ABI 兼容，内部以安全 Rust 重构。

**安全警告**: 此函数存在 TOCTOU 竞态条件，被 POSIX 标记为过时。但 ABI 兼容性要求仍须提供实现。

---

## 函数接口

```rust
use core::ffi::{c_char, c_int};

unsafe extern "C" fn tempnam(dir: *const c_char, pfx: *const c_char) -> *mut c_char;
```

[Visibility]: User — `<stdio.h>` POSIX 标准函数（XSI 扩展）。必须保持 ABI 兼容。返回值为 `malloc` 分配的动态字符串（调用者负责 `free`），NULL 表示失败并设置 `errno`。

---

### 前置/后置条件

**[Pre-condition]:**
- `dir`: 可为 NULL -> 默认使用 `P_tmpdir`（通常为 `"/tmp"`）。
- `pfx`: 可为 NULL -> 默认使用 `"temp"` 作为前缀。
- 路径总长度 `strlen(dir) + 1 + strlen(pfx) + 1 + 6` < `PATH_MAX`。

**[Post-condition]:**
- **Case 1 成功生成**
  - 返回指向唯一路径名字符串的指针（形式为 `<dir>/<pfx>_XXXXXX`）。
  - 该路径在设计上不与当前已有文件冲突。
  - 返回值由内部 `malloc` 分配，调用者负责 `free`。
  - 不创建文件。

- **Case 2 路径过长（长度 >= PATH_MAX）**
  - 返回 `core::ptr::null_mut()`。
  - `errno = ENAMETOOLONG`。

- **Case 3 所有尝试失败（100 次内无唯一文件名）**
  - 返回 `core::ptr::null_mut()`。
  - `errno` 为最后一次 `readlink` 系统调用的错误码。

**[Error Behavior]:**
- 路径过长返回 NULL + `ENAMETOOLONG`。
- 无法生成唯一文件名返回 NULL + 最后的 syscall 错误码。

---

### 不变量

**[Invariant]:**
- 生成的路径名始终为 `<dir>/<pfx>_XXXXXX` 格式。
- 最多 `MAXTRIES=100` 次尝试。
- 返回值由动态内存分配（`malloc`），调用者必须 `free`。
- 不创建任何文件。

---

### 意图

生成一个不与现有文件冲突的临时文件的路径名（不创建文件）。与 `tmpnam` 不同，允许调用者指定存放目录和文件名前缀。

Rust 侧实现：
- 外部接口使用 `unsafe extern "C" fn tempnam(dir: *const c_char, pfx: *const c_char) -> *mut c_char`，保持 ABI 兼容。
- 默认值处理：使用内部常量 `P_tmpdir`（`"/tmp"`）和默认前缀 `"temp"`。
- 路径组装：内部使用 Rust 安全字符串/字节操作计算长度和拼接路径。
- 路径长度校验通过 Rust 安全的长度计算完成。
- `__randname` 内部函数用 Rust 安全随机字符生成替代。
- 文件存在检测使用内部 syscall 模块封装的 `readlink`。
- `strdup` 替换为内部安全内存分配（调用 rusl 的 `malloc` + 字节复制，最终返回 `*mut c_char` 指针，布局与 C `malloc` 结果兼容）。
- 重试循环使用 Rust 迭代器风格实现。

### 系统算法

```
tempnam(dir, pfx):
  1. 默认值处理:
     若 dir == NULL: 使用 P_tmpdir         // 通常为 "/tmp"
     若 pfx == NULL: 使用 "temp"

  2. 长度校验:
     dl = strlen(dir)
     pl = strlen(pfx)
     l  = dl + 1 + pl + 1 + 6              // dir + '/' + pfx + '_' + XXXXXX
     若 l >= PATH_MAX:
       errno = ENAMETOOLONG
       返回 NULL

  3. 路径组装:
     s: [u8; PATH_MAX]
     写入: dir + '/' + pfx + '_'

  4. 循环 MAXTRIES=100 次:
       __randname(&mut s[l-6..l])           // 替换尾部 6 个字符
       r = readlink(s, &mut dummy, 1)       // 文件存在检测
       若 r == -ENOENT:                     // 路径不存在
         返回 strdup(s)                     // 动态分配并复制

  5. 返回 NULL  // 所有尝试失败
```

时间复杂度 O(PATH_MAX + MAXTRIES)，期望 O(PATH_MAX)。

---

## 依赖图

```
tempnam (Public, extern "C")
  ├── core::ffi::{c_char, c_int}                           — Rust 内置 FFI 类型
  ├── [Internal] __randname(buf: &mut [u8])                 — 内部随机文件名生成
  ├── [Internal] syscall 模块 (sys_readlink / sys_readlinkat) — 内部安全 syscall
  ├── [Internal] malloc / strdup                            — 内部内存分配模块
  ├── [Internal] strlen, memcpy                             — 内部字符串操作
  ├── [Internal] P_tmpdir, PATH_MAX, ENOENT, ENAMETOOLONG   — 平台常量
  └── [Internal] __errno_location()                          — 设置 errno 的入口
```

---

## [RELY]

- `core::ffi::{c_char, c_int}` — Rust 核心库 FFI 类型。
- 内部 `__randname` — rusl 内部安全 Rust 实现。
- 内部 syscall 模块 — rusl 内部实现，封装 readlink 系统调用。
- 内部 malloc — rusl 内部动态内存分配器。
- `__errno_location()` — rusl 内部 errno 访问器。

## [GUARANTEE]

Exported Interface:
  `unsafe extern "C" fn tempnam(dir: *const c_char, pfx: *const c_char) -> *mut c_char;`

本模块保证对外提供上述 ABI 兼容的函数符号：
- 参数类型布局与 C `const char *dir, const char *pfx` 完全一致。
- 返回值 `*mut c_char` 与 C `char *` 内存布局一致。
- 使用 C 调用约定 (`extern "C"`)。
- 行为符合 POSIX XSI `tempnam()` 语义。
- 返回值由 rusl `malloc` 分配，布局与 C `malloc` 兼容，调用者可安全 `free`。
