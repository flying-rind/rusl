# strdup — Rust 接口归约

## 原始 C 接口
```c
char *strdup(const char *s);
```

---

## Rust 外部 ABI 接口

```rust
extern "C" fn strdup(s: *const core::ffi::c_char) -> *mut core::ffi::c_char;
```

---

## 意图
创建字符串 s 的堆副本，调用者负责释放。

## 前置条件
- `s` 非空
- s 以 null 结尾

## 后置条件
- 返回 null 当且仅当分配失败
- 若成功，返回的字符串内容与 s 完全相同
- 调用者负责调用 free 释放

## 不变量
- 无全局或静态状态被修改

## 算法
分配 l+1 字节并复制内容：

```rust
pub fn strdup_impl(s: &core::ffi::CStr) -> Option<*mut u8> {
    let bytes = s.to_bytes_with_nul();
    let layout = core::alloc::Layout::array::<u8>(bytes.len()).ok()?;
    let ptr = unsafe { core::alloc::alloc(layout) as *mut u8 };
    if ptr.is_null() { return None; }
    unsafe { core::ptr::copy_nonoverlapping(bytes.as_ptr(), ptr, bytes.len()); }
    Some(ptr)
}
```

---

/* Rely */
[RELY]
Predefined Structures/Functions:
  core::ffi::CStr::to_bytes_with_nul  // 依赖1: 含 null 的字节切片
  core::alloc::Layout::array          // 依赖2: 内存布局
  core::alloc::alloc                  // 依赖3: 堆内存分配
  core::ptr::copy_nonoverlapping      // 依赖4: 非重叠复制

Predefined Macros/Traits:
  (none)

[GUARANTEE]
Exported Interface:
  extern "C" fn strdup(s: *const core::ffi::c_char) -> *mut core::ffi::c_char;
Internal Interface:
  pub(crate) fn strdup_impl(s: &core::ffi::CStr) -> Option<*mut u8>;