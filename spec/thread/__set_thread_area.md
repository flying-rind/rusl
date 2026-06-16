# __set_thread_area.c 规约

> musl libc 内部设置线程局部存储 (TLS) 区域指针的系统调用封装。在 x86 等架构上通过 `SYS_set_thread_area` 系统调用设置线程指针寄存器。

---

## 依赖图

```
__set_thread_area
  └─> __syscall(SYS_set_thread_area, p)   (see syscall.h — 系统调用)
```

---

## 函数规约

### 1. __set_thread_area

```c
int __set_thread_area(void *p);
```

[Visibility]: Internal (不导出) — 被 `pthread_impl.h` 声明为 hidden，仅 musl 内部使用

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
  #ifdef SYS_set_thread_area:
    return __syscall(SYS_set_thread_area, p)
  #else:
    return -ENOSYS
  #endif
```

#### 依赖

- `__syscall()` — 原始系统调用宏（见 `syscall.h`）
- `SYS_set_thread_area` — 系统调用号（架构相关）
