# strerror_r — Rust 接口归约

## 原始 C 接口
```c
int strerror_r(int err, char *buf, size_t buflen);
```

---

## Rust 外部 ABI 接口

```rust
extern "C" fn strerror_r(err: core::ffi::c_int, buf: *mut core::ffi::c_char, buflen: usize) -> core::ffi::c_int;
```

---

## 意图
将错误码 err 对应的错误描述字符串安全地复制到用户提供的缓冲区 buf 中（线程安全）。

## 前置条件
- `buf` 非空或 `buflen == 0`
- 当 `buflen > 0` 时，`buf` 至少可写 buflen 字节

## 后置条件
- 返回 0：buf 中包含完整的 null 终止错误消息
- 返回 ERANGE：缓冲区不足；若 buflen > 0，buf 包含截断的消息

## 不变量
- buf 末尾始终被正确终止（buflen > 0 时）
- 不会发生溢出

## 算法
获取消息并安全复制到缓冲区：

```rust
pub fn strerror_r_impl(err: core::ffi::c_int, buf: &mut [u8]) -> core::ffi::c_int {
    let msg = get_error_message(err);
    let msg_bytes = msg.as_bytes();
    if msg_bytes.len() >= buf.len() {
        if !buf.is_empty() {
            let copy_len = buf.len() - 1;
            buf[..copy_len].copy_from_slice(&msg_bytes[..copy_len]);
            buf[copy_len] = 0;
        }
        ERANGE
    } else {
        buf[..msg_bytes.len() + 1].copy_from_slice(&msg_bytes);
        0
    }
}
```

---

/* Rely */
[RELY]
Predefined Structures/Functions:
  core::slice::copy_from_slice  // 依赖1: 非重叠复制
  ERANGE                         // 依赖2: 缓冲区不足错误常量

Predefined Macros/Traits:
  (none)

[GUARANTEE]
Exported Interface:
  extern "C" fn strerror_r(err: core::ffi::c_int, buf: *mut core::ffi::c_char, buflen: usize) -> core::ffi::c_int;
Internal Interface:
  pub(crate) fn strerror_r_impl(err: core::ffi::c_int, buf: &mut [u8]) -> core::ffi::c_int;