# thrd_exit.c 规约

> musl libc 的 C11 线程退出函数实现。是 POSIX `__pthread_exit` 的转发包装器，将 C11 整型返回码转换为指针。

---

## 依赖图

```
thrd_exit
  └─> __pthread_exit((void*)(intptr_t)result)  — see pthread_impl.h (内部实现)
```

---

## 函数规约

### 1. thrd_exit

```c
_Noreturn void thrd_exit(int result);
```

[Visibility]: User — 通过 `<threads.h>` 对外导出 (ISO C11 7.26.5.2)

#### Intent

终止调用线程，将 `result` 设定为该线程的退出码。通过 `thrd_join` 获取此退出码。由于 POSIX 线程退出码为 `void *`，需将 `int` 通过 `intptr_t` 转换为 `void *`。函数标记为 `_Noreturn`，永不返回。

#### 前置条件

- `result` 为任意整型值（线程退出码）

#### 后置条件

- 调用线程终止，不再执行后续代码
- 线程的退出码可通过 `thrd_join` 以 `int` 形式获取
- 若存在 TSS 析构函数，按实现定义的行为执行
- 函数永不返回

#### 系统算法

```
thrd_exit(result):
  1. __pthread_exit((void*)(intptr_t)result)
     // intptr_t 往返转换确保 int -> void* -> int 无损
     // _Noreturn: 控制流永不回到调用者
```

#### 不变量

- `_Noreturn` 标记确保编译器知晓控制流终止

#### 依赖

- `__pthread_exit()` — POSIX 线程退出的内部实现（见 `pthread_impl.h`）
- `intptr_t` — 足以承载指针值的整数类型（见 `<stdint.h>`）
- `_Noreturn` — C11 函数属性，声明函数永不返回
