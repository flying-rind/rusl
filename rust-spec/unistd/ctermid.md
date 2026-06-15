# ctermid — Rust 接口归约

## 原始 C 接口
```c
char *ctermid(char *s);
```

---

## Rust 外部 ABI 接口

```rust
// 与 C ABI 兼容的底层导出函数
extern "C" fn ctermid(s: *mut core::ffi::c_char) -> *mut core::ffi::c_char;
```

---

## 意图

生成控制终端的路径名字符串。在 Linux/musl 上始终返回字符串 `"/dev/tty"`，因为 `/dev/tty` 是进程控制终端的通用引用路径。

## 前置条件

- `s`: 可为 `NULL` 或指向至少 `L_ctermid` 字节缓冲区的指针

## 后置条件

- **Case 1 `s` 不是 `NULL`**
  - 字符串 `"/dev/tty"` 被拷贝到 `s` 指向的缓冲区
  - 返回 `s`

- **Case 2 `s` 是 `NULL`**
  - 直接返回指向字符串字面量 `"/dev/tty"` 的指针
  - 返回的指针指向只读数据段

## 不变量

无。本函数不持有任何内部状态。

## 算法

原 C 实现直接返回 `strcpy(s, "/dev/tty")` 或字符串字面量。Rust 中：

```rust
extern "C" fn ctermid(s: *mut core::ffi::c_char) -> *mut core::ffi::c_char {
    if s.is_null() {
        // 返回指向只读字符串的指针
        c"/dev/tty".as_ptr() as *mut core::ffi::c_char
    } else {
        // 拷贝 "/dev/tty" 到用户缓冲区
        unsafe { copy_str_to_buf(c"/dev/tty", s) };
        s
    }
}
```

注意：musl 实现中 `"/dev/tty"` 是硬编码的字符串（8 字节 + null 终止符），这是一个满足 POSIX 标准的合法实现。POSIX 标准保证 `/dev/tty` 在所有符合标准的系统上都是控制终端的同义名。

---

## Rust 安全包装（模块内部）

```rust
/// 安全包装，返回 &'static str
pub(crate) fn control_terminal_path() -> &'static str {
    "/dev/tty"
}
```

---

/* Rely */
[RELY]
Predefined Structures/Functions:
  strcpy — 字符串拷贝（可用内部 copy_str_to_buf 替代）
  L_ctermid — 控制终端路径名最大长度宏（通常为 9）
Predefined Macros/Crates:
  无特殊依赖（纯字符串操作，可完全用 Rust 实现）

[GUARANTEE]
Exported Interface:
  extern "C" fn ctermid(s: *mut core::ffi::c_char) -> *mut core::ffi::c_char;
                                 // 本模块保证对外提供与 C ABI 兼容的 ctermid 符号
Internal Interface:
  pub(crate) fn control_terminal_path() -> &'static str;
                                 // 安全 Rust 包装
