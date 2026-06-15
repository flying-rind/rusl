# gethostname — Rust 接口归约

## 原始 C 接口
```c
int gethostname(char *name, size_t len);
```

---

## Rust 外部 ABI 接口

```rust
// 与 C ABI 兼容的底层导出函数
extern "C" fn gethostname(name: *mut core::ffi::c_char, len: usize) -> core::ffi::c_int;
```

---

## 意图

获取当前系统的主机名，写入用户提供的缓冲区 `name`。通过 `uname` 系统调用获取内核记录的 `nodename`（主机名），然后拷贝到用户缓冲区。当主机名超过缓冲区长度时，截断并确保 null 终止。

## 前置条件

- `name`: 指向至少 `len` 字节可用内存的非空指针
- `len`: 缓冲区大小，`> 0`

## 后置条件

- **Case 1 成功（主机名长度 < `len`）**
  - `name` 被写入以 null 结尾的完整主机名
  - 返回 0

- **Case 2 成功但被截断（主机名长度 >= `len`）**
  - `name` 被写入截断的主机名，`name[len-1]` 设置为 `'\0'`
  - 返回 0

- **Case 3 `uname` 系统调用失败**
  - 返回 -1
  - `errno` 由 `uname` 设置

## 不变量

无。本函数不持有任何内部状态。

## 算法

原 C 实现通过 `uname(&uts)` 获取系统信息后逐字节拷贝 nodename。Rust 中：

```rust
extern "C" fn gethostname(name: *mut core::ffi::c_char, len: usize) -> core::ffi::c_int {
    let mut uts = UtsName::default();
    // 1. 获取系统信息
    if unsafe { uname(&mut uts) } != 0 {
        return -1;
    }
    // 2. 限制拷贝长度不超过 nodename 字段大小
    let max_len = core::cmp::min(len, NODENAME_MAX_LEN);
    // 3. 逐字节拷贝 nodename 到 name
    let mut i = 0;
    while i < max_len {
        let ch = uts.nodename[i];
        unsafe { *name.add(i) = ch as core::ffi::c_char; }
        if ch == 0 {
            break;
        }
        i += 1;
    }
    // 4. 若未找到 null 终止符（截断情况），在末尾设置
    if i != 0 && i == len {
        unsafe { *name.add(i - 1) = 0; }
    }
    0
}
```

即：
1. 声明 `UtsName`，调用 `uname(&uts)` 获取内核系统信息。若失败，返回 -1
2. 计算最大拷贝长度：取 `len` 和 `uts.nodename` 字段大小的较小值（`uts.nodename` 通常为 65 字节）
3. 逐字节拷贝 `nodename` 到 `name`：
   - 这个循环同时进行拷贝和 null 检测
   - 如果 `nodename` 短于 `len`，会在 null 终止处停止
   - 如果到达 `len` 且该位置不是 null，循环在 `i == len` 时退出（缺少 null 终止符空间）
4. 若 `i != 0` 且 `i == len`（说明拷贝了 `len` 个非 null 字节），将最后一字节设为 `'\0'`

---

## Rust 安全包装（模块内部）

```rust
/// 安全包装，返回 Result<String, Errno>
/// 内部使用堆分配自动管理缓冲区大小
pub(crate) fn get_hostname() -> Result<String, Errno> {
    // 1. 调用 uname 获取系统信息
    // 2. 从 nodename 字段构造 Rust String
}
```

---

/* Rely */
[RELY]
Predefined Structures/Functions:
  uname — 获取系统名称信息
  struct utsname — 系统名称结构体，其 nodename 字段存储主机名（通常 65 字节）
Predefined Macros/Crates:
  syscall 模块 (rusl 内部 syscall 封装)

[GUARANTEE]
Exported Interface:
  extern "C" fn gethostname(name: *mut core::ffi::c_char, len: usize) -> core::ffi::c_int;
                                 // 本模块保证对外提供与 C ABI 兼容的 gethostname 符号
Internal Interface:
  pub(crate) fn get_hostname() -> Result<String, Errno>;
                                 // 安全 Rust 包装
