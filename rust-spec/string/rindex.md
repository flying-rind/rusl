# rindex — Rust 接口归约

## 原始 C 接口
```c
char *rindex(const char *s, int c);
```

---

## Rust 外部 ABI 接口

```rust
extern "C" fn rindex(s: *const core::ffi::c_char, c: core::ffi::c_int) -> *mut core::ffi::c_char;
```

---

## 意图
在字符串 s 中从后向前查找字符 c 最后一次出现的位置。等价于 strrchr。

## 前置条件
- `s` 非空
- s 指向以 null 结尾的有效 C 字符串

## 后置条件
- 若找到则返回指向 s 中最后一次匹配的指针
- 若未找到则返回 null
- s 内容不变

## 不变量
- 无全局或静态状态被修改

## 算法
直接委托给 strrchr：

```rust
pub fn rindex_impl(s: &core::ffi::CStr, c: u8) -> Option<*const core::ffi::c_char> {
    s.to_bytes().iter().rposition(|&b| b == c).map(|i| unsafe { s.as_ptr().add(i) })
}
```

---

/* Rely */
[RELY]
Predefined Structures/Functions:
  core::ffi::CStr::to_bytes         // 依赖1: 转为字节切片
  core::slice::<impl [u8]>::iter    // 依赖2: 字节迭代器
  core::iter::Iterator::rposition   // 依赖3: 反向位置查找

Predefined Macros/Traits:
  (none)

[GUARANTEE]
Exported Interface:
  extern "C" fn rindex(s: *const core::ffi::c_char, c: core::ffi::c_int) -> *mut core::ffi::c_char;
Internal Interface:
  pub(crate) fn rindex_impl(s: &core::ffi::CStr, c: u8) -> Option<*const core::ffi::c_char>;