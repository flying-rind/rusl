# strverscmp — Rust 接口归约

## 原始 C 接口
```c
int strverscmp(const char *l0, const char *r0);
```

---

## Rust 外部 ABI 接口

```rust
extern "C" fn strverscmp(l0: *const core::ffi::c_char, r0: *const core::ffi::c_char) -> core::ffi::c_int;
```

---

## 意图
比较两个字符串的"版本号顺序"（GNU 风格），以自然方式处理数字序列（"file1" < "file10"）。

## 前置条件
- `l0` 非空、`r0` 非空
- l0 和 r0 以 null 结尾

## 后置条件
- 返回 0：按版本号规则相等
- 返回 < 0：l0 在 r0 之前
- 返回 > 0：l0 在 r0 之后
- 字符串内容不变

## 不变量
- dp 标记最长匹配前缀中最后非数字字符位置
- z 标记数字后缀是否全为零

## 算法
找到最长匹配前缀后按版本号规则比较：

```rust
pub fn strverscmp_impl(l: &[u8], r: &[u8]) -> core::ffi::c_int {
    let mut dp = 0;
    let mut z = true;
    let (mut i, mut j) = (0, 0);
    while i < l.len() && j < r.len() && l[i] == r[j] && l[i] != 0 {
        if !l[i].is_ascii_digit() { dp = i + 1; z = true; }
        else if l[i] != b'0' { z = false; }
        i += 1; j += 1;
    }
    // 比较数字序列长度和值...
    0
}
```

---

/* Rely */
[RELY]
Predefined Structures/Functions:
  u8::is_ascii_digit  // 依赖1: 判断是否十进制数字

Predefined Macros/Traits:
  (none)

[GUARANTEE]
Exported Interface:
  extern "C" fn strverscmp(l0: *const core::ffi::c_char, r0: *const core::ffi::c_char) -> core::ffi::c_int;
Internal Interface:
  pub(crate) fn strverscmp_impl(l: &[u8], r: &[u8]) -> core::ffi::c_int;