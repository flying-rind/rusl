# wcsdup — Rust 接口归约

## 原始 C 接口
```c
wchar_t *wcsdup(const wchar_t *s);
```

---

## Rust 外部 ABI 接口

```rust
extern "C" fn wcsdup(s: *const u32) -> *mut u32;
```

---

## 意图
创建宽字符串 s 的堆副本。

## 前置条件
- `s` 非空
- s 以 L'\0' 结尾

## 后置条件
- 返回 null 当且仅当分配失败
- 若成功，返回与 s 相同的 null 终止宽字符串
- 调用者负责释放

## 不变量
- 无全局或静态状态被修改

## 算法
分配内存并复制宽字符串：

```rust
pub fn wcsdup_impl(s: &[u32]) -> Option<*mut u32> {
    let len = s.iter().position(|&c| c == 0).unwrap();
    let size = (len + 1) * core::mem::size_of::<u32>();
    let layout = core::alloc::Layout::from_size_align(size, core::mem::align_of::<u32>()).ok()?;
    let ptr = unsafe { core::alloc::alloc(layout) as *mut u32 };
    if ptr.is_null() { return None; }
    unsafe { core::ptr::copy_nonoverlapping(s.as_ptr(), ptr, len + 1); }
    Some(ptr)
}
```

---

/* Rely */
[RELY]
Predefined Structures/Functions:
  core::iter::Iterator::position    // 依赖1: 查找 null 位置
  core::alloc::Layout               // 依赖2: 内存布局
  core::alloc::alloc                // 依赖3: 堆内存分配
  core::ptr::copy_nonoverlapping    // 依赖4: 非重叠复制

Predefined Macros/Traits:
  (none)

[GUARANTEE]
Exported Interface:
  extern "C" fn wcsdup(s: *const u32) -> *mut u32;
Internal Interface:
  pub(crate) fn wcsdup_impl(s: &[u32]) -> Option<*mut u32>;