# strsignal — Rust 接口归约

## 原始 C 接口
```c
char *strsignal(int signum);
```

---

## Rust 外部 ABI 接口

```rust
extern "C" fn strsignal(signum: core::ffi::c_int) -> *mut core::ffi::c_char;
```

---

## 意图
返回信号编号 signum 对应的描述字符串。

## 前置条件
- 无（signum 可为任意整数）

## 后置条件
- 若 signum 有效（1 到 NSIG-1），返回信号描述字符串
- 若 signum 为 0 或无效，返回 "Unknown signal"
- 返回值不可被调用者修改

## 不变量
- 信号描述字符串表内容不变

## 算法
查表获取信号描述：

```rust
static SIGNAL_NAMES: &[&str] = &[
    "Unknown signal", "Hangup", "Interrupt", "Quit", "Illegal instruction",
    "Trace/breakpoint trap", "Aborted", "Bus error", "Floating point exception",
    // ...
];

pub fn strsignal_impl(signum: core::ffi::c_int) -> &'static core::ffi::CStr {
    let idx = signal_map(signum as usize);
    let name = SIGNAL_NAMES.get(idx).unwrap_or(&"Unknown signal");
    core::ffi::CStr::from_bytes_with_nul(name.as_bytes()).unwrap()
}
```

---

/* Rely */
[RELY]
Predefined Structures/Functions:
  core::ffi::CStr::from_bytes_with_nul  // 依赖1: 构造 C 字符串

Predefined Macros/Traits:
  NSIG  // 依赖2: 信号总数常量

[GUARANTEE]
Exported Interface:
  extern "C" fn strsignal(signum: core::ffi::c_int) -> *mut core::ffi::c_char;
Internal Interface:
  pub(crate) fn strsignal_impl(signum: core::ffi::c_int) -> &'static core::ffi::CStr;