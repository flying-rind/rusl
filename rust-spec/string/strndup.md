# strndup — Rust 接口归约

## 原始 C 接口
```c
char *strndup(const char *s, size_t n);
```

---

## Rust 外部 ABI 接口

```rust
extern "C" fn strndup(s: *const core::ffi::c_char, n: usize) -> *mut core::ffi::c_char;
```

---

## 意图
创建字符串 s 的副本，最多复制 n 个字符。通过 malloc 分配内存。

## 前置条件
- `s` 非空
- s 以 null 结尾

## 后置条件
- 返回 null 当且仅当分配失败
- 若成功，返回包含 s 前 l = min(strlen(s), n) 个字符的 null 终止字符串
- 调用者负责释放

## 不变量
- 无全局或静态状态被修改

## 算法
计算有界长度，分配并复制：

```rust
pub fn strndup_impl(s: &core::ffi::CStr, n: usize) -> Option<*mut u8> {
    let l = s.count_bytes().min(n);
    let layout = core::alloc::Layout::array::<u8>(l + 1).ok()?;
    let ptr = unsafe { core::alloc::alloc(layout) as *mut u8 };
    if ptr.is_null() { return None; }
    unsafe {
        core::ptr::copy_nonoverlapping(s.as_ptr(), ptr, l);
        *ptr.add(l) = 0;
    }
    Some(ptr)
}
```

---

/* Rely */
[RELY]
Predefined Structures/Functions:
  core::ffi::CStr::count_bytes     // 依赖1: 字符串长度
  core::alloc::Layout::array       // 依赖2: 内存布局
  core::alloc::alloc               // 依赖3: 堆内存分配
  core::ptr::copy_nonoverlapping   // 依赖4: 非重叠复制

Predefined Macros/Traits:
  (none)

[GUARANTEE]
Exported Interface:
  extern "C" fn strndup(s: *const core::ffi::c_char, n: usize) -> *mut core::ffi::c_char;
Internal Interface:
  pub(crate) fn strndup_impl(s: &core::ffi::CStr, n: usize) -> Option<*mut u8>;