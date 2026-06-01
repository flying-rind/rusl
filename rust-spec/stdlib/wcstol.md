# wcstol 族 Rust 接口

## 复杂度分级: Level 2

---

## Rust 接口

```rust
// [Visibility]: Public
#[no_mangle] pub unsafe extern "C" fn wcstol(s: *const wchar_t, endptr: *mut *mut wchar_t, base: i32) -> i64;
#[no_mangle] pub unsafe extern "C" fn wcstoll(s: *const wchar_t, endptr: *mut *mut wchar_t, base: i32) -> i64;
#[no_mangle] pub unsafe extern "C" fn wcstoul(s: *const wchar_t, endptr: *mut *mut wchar_t, base: i32) -> u64;
#[no_mangle] pub unsafe extern "C" fn wcstoull(s: *const wchar_t, endptr: *mut *mut wchar_t, base: i32) -> u64;
#[no_mangle] pub unsafe extern "C" fn wcstoimax(s: *const wchar_t, endptr: *mut *mut wchar_t, base: i32) -> i64;
#[no_mangle] pub unsafe extern "C" fn wcstoumax(s: *const wchar_t, endptr: *mut *mut wchar_t, base: i32) -> u64;
```

### 前置/后置条件

**[Visibility]:** Public

同 strtol 族，操作对象为宽字符串。内部委托给 __intscan。

### 系统算法

```
自定义 do_read: wchar_t -> byte -> FILE 适配 -> __intscan
```