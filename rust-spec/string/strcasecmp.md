# strcasecmp — Rust 接口归约

## 原始 C 接口
```c
int strcasecmp(const char *_l, const char *_r);
```

---

## Rust 外部 ABI 接口

```rust
extern "C" fn strcasecmp(_l: *const core::ffi::c_char, _r: *const core::ffi::c_char) -> core::ffi::c_int;
```

---

## 意图
忽略大小写比较两个 C 字符串 _l 和 _r。

## 前置条件
- `_l` 非空、`_r` 非空
- _l 和 _r 以 null 结尾

## 后置条件
- 返回 0：两字符串忽略大小写后相等
- 返回 < 0：在首个不同字符处 tolower(_l[i]) < tolower(_r[i])
- 返回 > 0：在首个不同字符处 tolower(_l[i]) > tolower(_r[i])
- 字符串内容不变

## 不变量
- 循环在遇到 null 或字符不相等时退出

## 算法
逐字节比较，每个字节通过 to_ascii_lowercase 转换后比较：

```rust
pub fn strcasecmp_impl(l: &[u8], r: &[u8]) -> core::ffi::c_int {
    for (a, b) in l.iter().zip(r.iter()) {
        let diff = a.to_ascii_lowercase() as i32 - b.to_ascii_lowercase() as i32;
        if diff != 0 || *a == 0 { return diff; }
    }
    0
}
```

---

/* Rely */
[RELY]
Predefined Structures/Functions:
  core::slice::<impl [u8]>::iter   // 依赖1: 字节迭代器
  u8::to_ascii_lowercase            // 依赖2: ASCII 小写转换

Predefined Macros/Traits:
  (none)

[GUARANTEE]
Exported Interface:
  extern "C" fn strcasecmp(_l: *const core::ffi::c_char, _r: *const core::ffi::c_char) -> core::ffi::c_int;
Internal Interface:
  pub(crate) fn strcasecmp_impl(l: &[u8], r: &[u8]) -> core::ffi::c_int;