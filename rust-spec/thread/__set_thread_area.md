# __set_thread_area — Rust 接口归约

> rusl 内部设置线程局部存储 (TLS) 区域指针的系统调用封装。在 x86 等架构上通过 `SYS_set_thread_area` 系统调用设置线程指针寄存器。

## 原始 C 接口

```c
int __set_thread_area(void *p);
```

[Visibility]: Internal — 被 `pthread_impl.h` 声明为 hidden，仅 musl/rusl 内部使用

---

## Rust 外部 ABI 接口

```rust
pub extern "C" fn __set_thread_area(p: *mut core::ffi::c_void) -> core::ffi::c_int;
```

---

## 依赖图

```
__set_thread_area
  └─> linux syscall: SYS_set_thread_area  (Linux 系统调用)
```

---

## 函数规约

### 1. __set_thread_area

```rust
pub extern "C" fn __set_thread_area(p: *mut core::ffi::c_void) -> core::ffi::c_int;
```

#### Intent

设置当前线程的 TLS 区域指针。通过 `SYS_set_thread_area` 系统调用将 TLS 描述符或线程区域基址传递给内核（主要用于 x86 架构）。在不支持此系统调用的架构上返回 `-ENOSYS`（TLS 的设置通过其他机制如 `ARCH_SET_FS` 或 ELF 辅助向量完成）。

#### 前置条件

- `p` 指向有效的 TLS 区域数据（格式依架构而定）
- 仅在支持 `SYS_set_thread_area` 的架构上编译时有效

#### 后置条件

- Case 支持 `SYS_set_thread_area`：调用系统调用设置 TLS，返回系统调用结果（0 成功，负数 errno 失败）
- Case 不支持 `SYS_set_thread_area`：返回 `-ENOSYS`

#### 系统算法

```
__set_thread_area(p):
  #[cfg(target_arch = "x86")]:
    return syscall(SYS_set_thread_area, p)
  #[cfg(not(target_arch = "x86"))]:
    return -ENOSYS
```

> Rust 实现中使用 `#[cfg(target_arch = ...)]` 条件编译替代 C 的 `#ifdef SYS_set_thread_area`。

#### 依赖

- `linux syscall: SYS_set_thread_area` — 系统调用（仅 x86 架构）
- `ENOSYS` — 错误码（定义于 errno）

---

## Rust 安全包装（模块内部，不对外暴露）

```rust
/// 内部设置 TLS 区域，返回 Result 以便错误处理
#[cfg(target_arch = "x86")]
pub(crate) fn set_thread_area(p: *mut core::ffi::c_void) -> Result<(), i32> {
    let ret = unsafe { syscall!(SYS_set_thread_area, p) };
    if ret < 0 { Err(ret as i32) } else { Ok(()) }
}

#[cfg(not(target_arch = "x86"))]
pub(crate) fn set_thread_area(_p: *mut core::ffi::c_void) -> Result<(), i32> {
    Err(ENOSYS)
}
```

---

/* Rely */
[RELY]
Predefined Structures/Functions:
  linux syscall: SYS_set_thread_area      // 依赖1: x86 架构 TLS 系统调用
  errno constants (ENOSYS)                // 依赖2: 错误码

[GUARANTEE]
Exported Interface:
  pub extern "C" fn __set_thread_area(p: *mut c_void) -> c_int;
                                         // 本模块保证对外提供与 C ABI 兼容的 __set_thread_area 符号
Internal Interface:
  pub(crate) fn set_thread_area(p: *mut c_void) -> Result<(), i32>;
                                         // 安全包装，供 crate 内部使用
