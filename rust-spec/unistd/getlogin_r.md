# getlogin_r — Rust 接口归约

## 原始 C 接口
```c
int getlogin_r(char *name, size_t size);
```

---

## Rust 外部 ABI 接口

```rust
// 与 C ABI 兼容的底层导出函数
extern "C" fn getlogin_r(name: *mut core::ffi::c_char, size: usize) -> core::ffi::c_int;
```

---

## 意图

获取当前登录用户的名字，写入用户提供的缓冲区 `name`。线程安全版本，使用用户分配的缓冲区而非内部静态变量。

## 前置条件

- `name`: 指向至少 `size` 字节可用内存的非空指针
- `size`: 缓冲区大小，`> 0`

## 后置条件

- **Case 1 成功**
  - `name` 缓冲区被写入以 null 结尾的登录用户名
  - 返回 0

- **Case 2 无法获取登录名**
  - 返回 `ENXIO`
  - `name` 内容未定义

- **Case 3 缓冲区太小（无法容纳登录名和 null 终止符）**
  - 返回 `ERANGE`
  - `name` 内容未定义

## 不变量

无。本函数不持有任何内部状态。

## 算法

原 C 实现通过 `getlogin()` 获取登录名、检查长度、使用 `strcpy` 拷贝到缓冲区。Rust 中：

```rust
extern "C" fn getlogin_r(name: *mut core::ffi::c_char, size: usize) -> core::ffi::c_int {
    // 1. 获取登录名
    let login_ptr = unsafe { getlogin() };
    if login_ptr.is_null() {
        return ENXIO;
    }
    // 2. 计算长度
    let login_len = unsafe { strlen(login_ptr) };
    // 3. 检查缓冲区大小（需要额外一个字节放 null）
    if login_len >= size {
        return ERANGE;
    }
    // 4. 拷贝到用户缓冲区
    unsafe { copy_str_to_buf_from_ptr(login_ptr, name, login_len + 1) };
    0
}
```

即：
1. 调用 `getlogin()` 获取登录名（环境变量 `LOGNAME` 的值）
2. 若返回 `NULL`（环境变量不存在），返回 `ENXIO` 错误码
3. 检查用户名长度：若 `strlen(logname) >= size`，返回 `ERANGE`（注意：条件是 `>=` 而非 `>`，因为需要额外一个字节存放 null 终止符）
4. 将用户名拷贝到用户缓冲区
5. 返回 0 表示成功

---

## Rust 安全包装（模块内部）

```rust
/// 安全包装，返回 Result<String, Errno>
/// 使用堆分配自动管理缓冲区大小，避免缓冲区太小的问题
pub(crate) fn get_login_name() -> Result<String, Errno> {
    // 先获取登录名字符串切片
    let name = login_name().ok_or(ENXIO)?;
    Ok(name.to_owned()) // 复制到堆上，返回独立 String
}
```

---

/* Rely */
[RELY]
Predefined Structures/Functions:
  getlogin — 非线程安全版登录名获取
  strlen — 字符串长度计算（可用 Rust 内部替代）
  copy_str_to_buf — 字符串拷贝到缓冲区（可用 Rust 内部替代）
  ENXIO — "无此设备或地址" 错误码
  ERANGE — 结果超出范围错误码
Predefined Macros/Crates:
  env 模块 (rusl 内部环境变量管理)

[GUARANTEE]
Exported Interface:
  extern "C" fn getlogin_r(name: *mut core::ffi::c_char, size: usize) -> core::ffi::c_int;
                                 // 本模块保证对外提供与 C ABI 兼容的 getlogin_r 符号
Internal Interface:
  pub(crate) fn get_login_name() -> Result<String, Errno>;
                                 // 安全 Rust 包装
