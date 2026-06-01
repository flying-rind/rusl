# getenv.rs 规约

## 依赖图

```
getenv
├── __strchrnul  (依赖 — 来自 src/string/ 模块，内部符号)
├── strncmp      (依赖 — 来自 src/string/ 模块，标准 C 库函数)
└── environ      (依赖 — 来自 src/env/__environ.rs，POSIX 标准全局变量)
```

---

## 原始 C 接口

```c
char *getenv(const char *name);
```

---

## Rust 外部 ABI 接口

```rust
// [Visibility]: Public — POSIX.1-2001 标准函数，<stdlib.h> 声明。用户程序可直接调用。
#[no_mangle]
pub extern "C" fn getenv(name: *const c_char) -> *mut c_char;
```

**ABI 兼容性说明:**

| C 签名 | Rust 签名 |
|--------|-----------|
| `char *getenv(const char *name)` | `pub extern "C" fn getenv(name: *const c_char) -> *mut c_char` |

- 参数 `name: *const c_char` 对应 `const char *name`，常量指针语义。
- 返回值 `*mut c_char` 对应 `char *`，调用者只能读取返回的内存，不应修改或释放。

---

## 意图 (Intent)

在进程环境变量列表中查找指定名称的环境变量，返回其对应的值字符串。搜索使用精确名称匹配，遵循 POSIX 语义：musl 实现严格区分大小写（符合 POSIX 标准允许的行为）。

---

## 前置条件 (Preconditions)

| 条件 | 说明 |
|------|------|
| `name != core::ptr::null()` | 调用者必须传入有效的 C 字符串指针 |
| `*name` 是以 `'\0'` 结尾的合法字符串 | 标准 C 字符串约束 |
| `name` 指向的字符串中不包含 `'='` 字符 | POSIX 标准规定：环境变量名不得包含 `=`；若 `name` 含 `=`，函数将返回 `null_mut()`（视为未找到） |
| `name` 指向的字符串长度 > 0 | 空字符串 `""` 不是合法的环境变量名，返回 `null_mut()` |

---

## 后置条件 (Postconditions)

**Case 1 — 找到匹配的环境变量:**

| 条件 | 结果 |
|------|------|
| `environ != core::ptr::null_mut()` | 环境块已初始化 |
| 存在 `environ[i]`，使得 `strncmp(name, environ[i], l) == 0` 且 `environ[i][l] == '='`（其中 `l` 为名称长度） | 名称匹配且后接 `=` 分隔符 |
| 返回值 | 指向 `environ[i] + l + 1` 的指针，即值字符串的起始地址 |
| 返回值有效性 | 指向的字符串位于进程环境内存中，调用者**不得修改或释放**该内存 |
| 线程安全性 | musl 的 `getenv` 不持有锁，读操作本身是数据竞争安全的（读取指针），但若其他线程同时调用 `putenv`/`setenv`/`unsetenv`，行为未定义 |

**Case 2 — 未找到环境变量:**

| 条件 | 结果 |
|------|------|
| `environ == core::ptr::null_mut()` | 环境块未初始化，直接返回 `null_mut()` |
| 或 `l == 0`（`name` 为空字符串） | 返回 `null_mut()` |
| 或 `name[l] == '='`（`name` 含 `=` 字符） | 返回 `null_mut()`（被视为非法变量名） |
| 或遍历整个 `environ` 数组后无匹配项 | 返回 `null_mut()` |

---

## 不变量 (Invariants)

- `getenv` 返回的指针在下次修改环境的调用（`putenv`、`setenv`、`unsetenv`、`clearenv`）之前保持有效；修改环境后，该指针可能指向已被替换或释放的内存。
- 若多次以相同 `name` 调用 `getenv` 且期间未修改环境，每次返回值相同。
- 该函数不设置 `errno`。

---

## 系统算法 (System Algorithm)

```
function getenv(name: *const c_char) -> *mut c_char:
    // 计算 name 长度，同时检测是否包含 '='
    let eq_or_end = __strchrnul(name, '=' as c_int);
    let l = (eq_or_end as usize) - (name as usize);

    // 非法名称: 空字符串 或 包含 '='
    if l == 0 || unsafe { *eq_or_end } != 0 {
        return core::ptr::null_mut();
    }

    // 环境块未初始化
    let env_ptr = unsafe { environ };
    if env_ptr.is_null() {
        return core::ptr::null_mut();
    }

    // 线性扫描环境变量数组
    let mut i = 0;
    loop {
        let entry = unsafe { *env_ptr.add(i) };
        if entry.is_null() {
            break;  // 到达数组末尾
        }
        // 检查名称匹配: 前 l 个字符相同，且第 l 个字符为 '='
        if unsafe { strncmp(name, entry, l) } == 0
            && unsafe { *entry.add(l) } == '=' as u8
        {
            return unsafe { entry.add(l + 1) };
        }
        i += 1;
    }

    core::ptr::null_mut()
```

**算法要点:**

1. **名称校验**: 使用 `__strchrnul` 一次性完成长度计算与 `=` 字符检测，避免两次扫描。`__strchrnul` 一次遍历定位 `=` 或字符串末尾，比单独调用 `strlen` + `strchr` 更高效。
2. **惰性 NULL 检查**: `environ` 在循环外检查一次，避免在每次迭代中重复检查。
3. **逐项线性扫描**: 对环境变量数组进行线性搜索，时间复杂度 O(n * m)，其中 n 为环境变量数目，m 为名称长度。POSIX 标准允许此实现复杂度。
4. **精确名称匹配**: 使用 `strncmp` 比较前 l 个字符，再检查第 l 个字符是否为 `=` —— 防止将 `NAME` 错误匹配到 `NAMEOTHER=value`。
5. **返回值语义**: 返回的是指向环境内存内部的指针，而非新分配的副本。调用者读取此指针是安全的，但不应修改或释放。

---

## 内部实现设计要点

- `environ` 来自同模块的 `__environ.rs`，类型为 `pub extern "C" static mut environ: *mut *mut c_char`，初始值为 `core::ptr::null_mut()`。`getenv` 通过模块私有途径访问该变量。
- `__strchrnul` 作为内部辅助函数，可在 Rust 侧用安全抽象重写（例如使用 `core::ffi::CStr` 和字节切片操作，或通过 `memchr` crate 实现），但必须保持相同的语义：扫描字符串中首次出现的指定字符，若找到返回指向该字符的指针，否则返回指向终止 `'\0'` 的指针。
- `strncmp` 为标准 C 库函数，Rust 侧可通过 `core::ffi::CStr` 的字节比较或手写循环实现等效功能，无需单独调用 C 版本的 `strncmp`。
- 由于 `environ` 为 `static mut`，所有访问需通过 `unsafe` 块进行裸指针操作。内部实现应将 `unsafe` 范围限制在必要的指针读写上，不将大段逻辑包裹在 `unsafe` 中。
- 可设计内部安全辅助函数 `fn find_env_value(name: &CStr) -> Option<*mut c_char>`，将名称校验和遍历逻辑封装在安全 Rust 中，`getenv` 仅负责裸指针与 `CStr` 的边界转换。

---

## 相关文件与依赖关系

| 模块 | 关系 | 说明 |
|------|------|------|
| `src/env/__environ.rs` | 依赖 | 提供 `environ` 全局变量，`getenv` 遍历其指向的环境变量数组 |
| `src/string/strchrnul.rs` | 依赖 | 提供 `__strchrnul` 内部函数，用于名称长度计算和 `=` 检测 |
| `src/string/strncmp.rs` | 依赖 | 提供 `strncmp` 标准函数，用于名称前缀精确匹配 |
| `src/env/setenv.rs` | 同级模块 | 修改环境时可能重新分配 `environ` 数组，使 `getenv` 已有返回值失效 |
| `src/env/putenv.rs` | 同级模块 | 直接替换 `environ` 数组中的指针 |
| `src/env/unsetenv.rs` | 同级模块 | 从 `environ` 数组中移除条目 |
| `src/env/clearenv.rs` | 同级模块 | 将 `environ` 置为 `null_mut()` |

---

```
/* Rely */

[RELY]
Predefined Structures/Functions:

  // 依赖1: __strchrnul — 来自 src/string/ 模块
  //   扫描字符串 s 中首次出现的字符 c，返回指向该字符的指针；
  //   若未找到，返回指向终止 '\0' 的指针。
  //   用于一次性完成名称长度计算与 '=' 字符检测。
  //   [Visibility]: Internal
  pub(crate) fn __strchrnul(s: *const c_char, c: c_int) -> *const c_char;

  // 依赖2: strncmp — 来自 src/string/ 模块，标准 C 库函数
  //   比较 s1 和 s2 的前 n 个字符，返回比较结果。
  //   [Visibility]: Public
  pub extern "C" fn strncmp(s1: *const c_char, s2: *const c_char, n: size_t) -> c_int;

  // 依赖3: environ — 来自 src/env/__environ.rs 模块
  //   POSIX 标准全局变量，指向以 null 指针结尾的环境变量字符串数组。
  //   每个元素为 "NAME=VALUE" 格式的 C 字符串。
  //   初始值为 core::ptr::null_mut()，由启动代码在 main() 之前填充。
  //   [Visibility]: Public (通过 extern "C" static mut 导出为 POSIX environ 符号)
  pub extern "C" static mut environ: *mut *mut c_char;

```

---

```
/* Guarantee */

[GUARANTEE]
Exported Interface:

  // 函数: getenv (POSIX 标准对外导出)
  #[no_mangle]
  pub extern "C" fn getenv(name: *const c_char) -> *mut c_char;
  //   在进程环境变量列表中查找指定名称的环境变量。
  //   - name: 要查找的环境变量名称（C 字符串指针，不得为 NULL，不得包含 '='）
  //   - 返回值: 指向值字符串的指针；若未找到则返回 core::ptr::null_mut()
  //   - 返回的指针指向进程环境内存，调用者不可修改或释放
  //   - 本模块保证上述签名和语义在所有支持的平台上保持 ABI 兼容

Internal Interface:

  // getenv 的内部辅助函数（模块私有，不对外暴露）
  pub(crate) fn find_env_value(name: &core::ffi::CStr) -> Option<*mut c_char>;
  //   安全 Rust 封装：接收已验证的 CStr 引用，遍历 environ 数组进行名称匹配。
  //   - name: 已校验的环境变量名（不含 '='，非空）
  //   - 返回值: Some(ptr) 表示找到匹配项，ptr 指向值字符串；None 表示未找到
  //   - 该函数内部使用 unsafe 块访问 environ，但 unsafe 范围仅限于必要的指针操作
```
