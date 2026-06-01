# unsetenv — Rust 接口归约

## 依赖图

```
environ (外部, 来自 __environ 模块, Public)
  └── unsetenv ──> CStr::to_bytes_with_nul / to_bytes
               │     // 替代 C 的 __strchrnul + strncmp，
               │     // 使用 Rust core CStr 安全抽象进行键名校验和条目匹配
               ├──> __env_rm_add (可替换函数指针, 默认指向 dummy)
               │     └──> dummy (模块私有, 空实现)
               └──> set_errno / EINVAL (内部 errno 机制)

__env_rm_add ──> dummy (默认无操作)
              └──> setenv 模块强实现 (运行时注册, 覆盖默认值)
```

---

## 原始 C 接口
```c
int unsetenv(const char *name);
```

---

## Rust 外部 ABI 接口

```rust
extern "C" fn unsetenv(name: *const core::ffi::c_char) -> core::ffi::c_int;
```

[Visibility]: External — POSIX 标准函数，声明于 `<stdlib.h>`，必须保持与 C ABI 完全兼容。

---

## 意图 (Intent)

从进程环境变量列表中移除指定名称的环境变量。函数对 `environ` 指针数组执行**原地压缩（in-place compaction）**：遍历数组中的每个条目，将匹配 `"name=value"` 的条目移除（通过 `__env_rm_add` 回调通知内存管理模块），并将不匹配的条目向数组头部移动以填补空隙。该算法在一次遍历中同时完成查找和压缩，时间复杂度 O(n)，空间复杂度 O(1)。

Rust 实现的核心策略：

- **外部 ABI 层 (`unsetenv`)**: 保持 `extern "C"` ABI 兼容，接收原始 C 指针。
- **内部安全层 (`unsetenv_impl`)**: 接收 `&CStr` 引用，使用 `CStr` 方法替代 C 的 `__strchrnul` + `strncmp` 进行键名校验和条目匹配，仅在访问全局 `environ` 和操作裸指针时使用最小范围的 `unsafe` 块。

---

## 前置条件

- `name` 非 NULL（调用者保证；外部 ABI 函数在入口处通过 `unsafe` 解引用转换为 `&CStr`）
- `name` 非空字符串（有效键名长度 `l > 0`），且不包含 `=` 字符（`name_bytes[l] == b'\0'`）
- `environ` 若为非 NULL，则为以 NULL 终止的有效 `*mut c_char` 数组
- 调用上下文非多线程环境（POSIX 规定 `setenv`/`unsetenv`/`putenv` 非线程安全）

---

## 后置条件

| Case | 条件 | 返回值 | errno | `environ` 状态 |
|------|------|--------|-------|-------------------|
| **Case 1** | `name` 为合法环境变量名且 `environ` 中存在匹配项 | 0 | 不变 | `environ` 数组被压缩，匹配项被移除，末尾补 NULL |
| **Case 2** | `name` 为合法环境变量名但 `environ` 中无匹配项 | 0 | 不变 | `environ` 不变 |
| **Case 3** | `name` 为空字符串（`l == 0`） | -1 | `EINVAL` | `environ` 不变 |
| **Case 4** | `name` 包含 `=` 字符 | -1 | `EINVAL` | `environ` 不变 |
| **Case 5** | `environ` 为 NULL 且 `name` 合法 | 0 | 不变 | 无操作（循环体不执行） |

---

## 不变量 (Invariants)

**循环不变量**：
- 在每次循环迭代开始前，`writer <= reader`（写指针不超过读指针）
- `environ[0..writer)` 中均为已完成处理的非匹配条目（即不包含键名为 `name` 的环境变量）
- `environ[writer..reader)` 中的条目已被丢弃或将被后续保留项覆盖

**模块级不变量**：
- 若 `unsetenv` 返回 0，`environ` 中不存在键名为 `name` 的环境变量条目（幂等性保证）
- 函数不分配任何内存，也不释放除通过 `__env_rm_add` 委托之外的内存

---

## 系统算法 — 单趟双指针原地压缩 (Level 3)

Rust 内部实现在入口处将 `*const c_char` 参数转换为 `&CStr` 引用，利用 `CStr` 的字节切片方法替代 C 的 `__strchrnul` 和 `strncmp`，仅在最终访问 `environ` 全局指针和执行指针复制时进入 `unsafe` 块。

**输入**: `name: &core::ffi::CStr` （环境变量键名引用）

```
1.  将 name 转为含 NUL 的字节切片:
        name_bytes = name.to_bytes_with_nul()
2.  查找 '=' 的位置:
        l = name_bytes.iter().position(|&b| b == b'=').unwrap_or(name_bytes.len() - 1)
        // l = 键名长度（若 name 中无 '='）或首个 '=' 的索引
3.  验证 name:
        若 l == 0 (空字符串) 或 name_bytes[l] != b'\0' (包含 '='):
            设置 errno = EINVAL, 返回 -1
4.  读取 environ 指针 (unsafe):
        若 environ 为 null_mut(): 返回 0 (无操作)
5.  初始化双指针:
        reader = environ
        writer = environ
        // reader: 遍历所有条目
        // writer: 指向下一个保留条目的写入位置
6.  遍历: 对每个 *reader (直到 *reader 为 null):
    6a. 获取 entry_cstr = CStr::from_ptr(*reader)
    6b. 获取 entry_bytes = entry_cstr.to_bytes()
    6c. 判断匹配:
            若 entry_bytes.len() > l
              且 &entry_bytes[..l] == &name_bytes[..l]
              且 entry_bytes[l] == b'='
            则视为匹配:
                调用 __env_rm_add(*reader, null_mut())
                // writer 不动（跳过此项，产生间隙）
    6d. 否则（不匹配）:
            若 writer != reader: *writer = *reader  // 向前移动保留条目
            writer = writer.add(1)
    6e. reader = reader.add(1)
7.  若 writer != reader (有项被移除): *writer = null_mut() (重新终止数组)
8.  返回 0
```

**指针状态说明**:

| 状态 | reader（读指针） | writer（写指针） | 含义 |
|------|-----------------|------------------|------|
| 初始 | 指向 `environ[0]` | 指向 `environ[0]` | writer == reader，无间隙 |
| 匹配后 | 前进到下一项 | 停留在当前项 | writer < reader，存在间隙，后续保留项需前移 |
| 不匹配+无间隙 | 前进 | 前进 | 同步移动，无需复制 |
| 不匹配+有间隙 | 前进 | 复制后前进 | `*writer = *reader` 将保留项移动到间隙处 |

---

## 内部实现伪代码

```rust
/// 安全抽象实现，接收 CStr 引用而非裸指针
pub(crate) unsafe fn unsetenv_impl(name: &core::ffi::CStr) -> core::ffi::c_int {
    let name_bytes = name.to_bytes_with_nul();

    // 查找 '=' 的位置，若不存在则为 name 长度（不含末尾 NUL）
    let l = name_bytes.iter()
        .position(|&b| b == b'=')
        .unwrap_or(name_bytes.len() - 1);

    // 验证 name: 非空且不包含 '='
    if l == 0 || name_bytes[l] != 0 {
        crate::errno::set_errno(EINVAL);
        return -1;
    }

    // 若环境变量数组为空，直接返回
    if environ.is_null() {
        return 0;
    }

    let mut reader = environ;
    let mut writer = environ;

    // 单趟双指针原地压缩
    while !(*reader).is_null() {
        let entry_ptr = *reader;
        let entry_cstr = core::ffi::CStr::from_ptr(entry_ptr as *const _);
        let entry_bytes = entry_cstr.to_bytes();

        // 判断条目是否匹配要删除的键名
        let is_match = entry_bytes.len() > l
            && &entry_bytes[..l] == &name_bytes[..l]
            && entry_bytes[l] == b'=';

        if is_match {
            // 通知内存管理模块：该条目即将被移除
            __env_rm_add(entry_ptr, core::ptr::null_mut());
            // writer 不动，reader 前进，产生间隙
        } else {
            // 保留此条目，必要时前移
            if writer != reader {
                *writer = entry_ptr;
            }
            writer = writer.add(1);
        }
        reader = reader.add(1);
    }

    // 若有项被移除，用 NULL 重新终止数组
    if writer != reader {
        *writer = core::ptr::null_mut();
    }

    0
}

/// 外部 ABI 包装：将裸指针转换为 CStr 引用后委托给 unsetenv_impl
#[no_mangle]
pub extern "C" fn unsetenv(name: *const core::ffi::c_char) -> core::ffi::c_int {
    // SAFETY: 调用者保证 name 为有效的 NUL 终止 C 字符串指针
    if name.is_null() {
        crate::errno::set_errno(EINVAL);
        return -1;
    }
    let name_cstr = unsafe { core::ffi::CStr::from_ptr(name) };
    unsafe { unsetenv_impl(name_cstr) }
}
```

---

## `__env_rm_add` 跨模块通信机制

在 C 实现中，`__env_rm_add` 是一个 `weak_alias` 弱符号——若 `setenv.c` 参与链接则解析为强定义，否则解析为无操作的 `dummy`。在 Rust `#![no_std]` 环境中无法直接使用弱符号，因此改用**可替换函数指针**模式：

```rust
/// 环境变量内存管理回调函数类型：
/// - old: 被替换或移除的旧环境变量指针（可能为 null）
/// - new: 新分配的环境变量字符串指针（可能为 null）
type EnvRmAddFn = fn(old: *mut core::ffi::c_char, new: *mut core::ffi::c_char);

/// 全局可替换回调，默认为无操作实现
/// 当 setenv 模块初始化时，将其替换为实际的内存管理实现
static mut __env_rm_add: EnvRmAddFn = dummy;

/// 默认无操作实现（等效于原 C 代码中的 `static void dummy(char *old, char *new) {}`）
fn dummy(_old: *mut core::ffi::c_char, _new: *mut core::ffi::c_char) {}
```

**设计理由**:
- `static mut` 函数指针可在运行时被替换，等价于 C 链接期的弱符号覆盖行为
- 未使用 `setenv` 时，`__env_rm_add` 保持为 `dummy`（零开销——编译器可内联空函数体）
- 使用 `setenv` 时，`setenv` 模块在初始化时将 `__env_rm_add` 替换为自己的实现
- 该机制仅用于模块内部通信，`dummy` 和 `__env_rm_add` 均为 `pub(crate)` 或更小可见性

---

## 写入策略

函数对 `environ` 数组进行原地修改：
- 匹配项的位置被后续保留项覆盖（通过 `*writer = entry_ptr`）
- 仅当 `writer != reader`（即有项被移除）时才写入新的 NULL 终止符，避免不必要的写入
- 该实现是线程不安全的（符合 POSIX 语义——`setenv`/`unsetenv`/`putenv` 非线程安全）
- 所有对 `environ` 的读写集中在最小范围的 `unsafe` 块内，其余逻辑使用安全 Rust

---

## 跨文件交互说明

`unsetenv` 调用 `__env_rm_add(*reader, null_mut())` 来通知已分配字符串的管理模块：该环境变量条目即将从 `environ` 中移除。由于第二个参数为 `null_mut()`（NULL），表示没有新的替换字符串。

- **若 setenv 模块已初始化**：`__env_rm_add` 已被替换为实际实现，将释放 `*reader` 指向的堆分配内存并清理登记表
- **若 setenv 模块未初始化**：`__env_rm_add` 仍为默认的 `dummy` 无操作——此时 `*reader` 可能指向只读的启动环境字符串，不应释放

### 与 setenv / putenv 的协作语义

| 模块 | `__env_rm_add` 调用方式 | 说明 |
|------|--------------------------|------|
| **unsetenv** | `__env_rm_add(*e, null_mut())` | 移除匹配条目，无替换字符串 |
| **putenv** | `__env_rm_add(old, null_mut())` | old 为被替换的旧条目，无新分配 |
| **setenv** | `__env_rm_add(old, new)` 或 `__env_rm_add(null_mut(), r)` | 替换时同时释放旧值并注册新值；新增时注册新分配字符串 |

---

## 与 C 实现差异说明

| 项目 | C 实现 | Rust 实现 |
|------|--------|-----------|
| 弱符号机制 (`__env_rm_add`) | ELF `weak_alias` 编译期链接覆盖 | `static mut` 函数指针，运行期注册 |
| 键名校验 (`__strchrnul`) | 跨模块内部函数调用 | `CStr::to_bytes_with_nul()` + `Iterator::position()` |
| 条目匹配 (`strncmp`) | 标准库字符串比较 | `&[u8]` 切片比较 (`&entry_bytes[..l] == &name_bytes[..l]`) |
| 环境数组访问 | `extern char **__environ` | 通过 `environ` 全局变量直接访问（`__environ.md` 中定义为 `pub static mut environ`） |
| 内部函数可见性 | 文件内 `static` 或跨模块 `__` 前缀 | `pub(crate)` 或更小可见性 |
| 安全抽象 | 全裸指针操作 | 入口处转换为 `&CStr`，内部使用切片比较，仅在访问 `environ` 时使用 `unsafe` |

---

/* Rely */
[RELY]
Predefined Structures/Functions:
  environ: *mut *mut core::ffi::c_char
    // 依赖1: 全局环境变量指针数组，以 NULL 终止（定义于 src/env/__environ 模块，
    //        符号名为 `environ`，类型等价于 C 的 `char **`）
  __env_rm_add(old: *mut core::ffi::c_char, new: *mut core::ffi::c_char)
    // 依赖2: 环境变量字符串生命周期管理回调（默认指向 dummy 无操作，
    //        可被 setenv 模块在运行时替换为其实内存管理实现）
  crate::errno::set_errno(err: core::ffi::c_int)
    // 依赖3: 设置 errno 值，用于向调用者报告 EINVAL 错误
  core::ffi::CStr::from_ptr(ptr: *const core::ffi::c_char) -> &core::ffi::CStr
    // 依赖4: 从裸指针构造 CStr 引用（unsafe，调用者保证指针有效且 NUL 终止）
  core::ffi::CStr::to_bytes(&self) -> &[u8]
    // 依赖5: 获取 C 字符串的字节切片（不含末尾 NUL），用于条目匹配比较
  core::ffi::CStr::to_bytes_with_nul(&self) -> &[u8]
    // 依赖6: 获取 C 字符串的字节切片（含末尾 NUL），用于键名长度计算和 '=' 检测
  core::iter::Iterator::position(|&u8| -> bool) -> Option<usize>
    // 依赖7: 迭代器位置查找，用于在 name 字节切片中定位 '=' 字符（替代 __strchrnul）
  core::ptr::null_mut::<T>() -> *mut T
    // 依赖8: 生成空指针，用于终止环境数组和作为 __env_rm_add 的空参数
  core::ptr::Pointer::is_null(&self) -> bool
    // 依赖9: 判断指针是否为 null
  core::ptr::Pointer::add(&self, count: usize) -> *mut T
    // 依赖10: 指针偏移，用于读写指针的前进操作

Predefined Macros/Traits:
  core::cmp::PartialEq (for &[u8] slice comparison)
    // 依赖11: 字节切片相等性比较，用于替代 C 的 strncmp 进行键名匹配

[GUARANTEE]
Exported Interface:
  extern "C" fn unsetenv(name: *const core::ffi::c_char) -> core::ffi::c_int;
    // 本模块保证对外提供的 ABI 兼容接口
    // 声明于 <stdlib.h>，POSIX.1-2001 标准函数
    // 参数 name: 要移除的环境变量名（NUL 终止 C 字符串）
    // 返回 0 表示成功（变量已移除或不存在），-1 表示参数无效（errno = EINVAL）

Internal Interface:
  pub(crate) unsafe fn unsetenv_impl(name: &core::ffi::CStr) -> core::ffi::c_int;
    // 内部安全抽象实现，接受 CStr 引用而非裸指针
    // 调用者保证 environ 和 name 满足前置条件
    // unsafe 标记因其访问全局可变静态变量 environ 并执行裸指针写入

  type EnvRmAddFn = fn(old: *mut core::ffi::c_char, new: *mut core::ffi::c_char);
    // 环境变量内存管理回调函数类型
    // old: 被移除或替换的旧环境字符串指针
    // new: 替换字符串指针（本模块始终传 null_mut()）

  static mut __env_rm_add: EnvRmAddFn;
    // 可替换的全局回调静态变量，默认指向 dummy 无操作实现
    // setenv 模块可通过直接赋值替换此指针以注册其内存管理逻辑

  fn dummy(_old: *mut core::ffi::c_char, _new: *mut core::ffi::c_char);
    // 默认无操作回调实现（模块私有，等效于原 C 代码中的 static void dummy(char*, char*){}）
