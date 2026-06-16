# default_attr.c 规约

> musl libc 线程属性默认值定义。定义两个模块内部全局变量，保存创建新线程时的默认栈大小和守护页大小。

---

## 依赖图

```
__default_stacksize  (全局变量)
  └─> DEFAULT_STACK_SIZE   (宏, pthread_impl.h)

__default_guardsize  (全局变量)
  └─> DEFAULT_GUARD_SIZE   (宏, pthread_impl.h)
```

---

## 全局变量规约

### 1. __default_stacksize

```c
unsigned __default_stacksize = DEFAULT_STACK_SIZE;
```

[Visibility]: Internal (不导出) — musl 线程模块内部共享的全局变量，被 `pthread_attr_init`、`pthread_setattr_default_np`、`pthread_getattr_default_np` 等引用。

#### Intent

保存新创建线程的默认栈大小。该值在启动时初始化为 `DEFAULT_STACK_SIZE`（131072 = 128KB），可通过 `pthread_setattr_default_np` 增大（不可减小），始终被限制在 `DEFAULT_STACK_MAX`（8MB）以内。

#### 前置条件

无（初始化时由编译期常量赋值）。

#### 后置条件

- 初始值为 `131072`
- 后续修改需通过 `__inhibit_ptc` / `__release_ptc` 保护
- 值始终 >= 初始值，且 <= `DEFAULT_STACK_MAX`

#### 不变量

- 任何 `__default_stacksize` 修改必须在持有 PTC 锁时进行
- 值单调不减

---

### 2. __default_guardsize

```c
unsigned __default_guardsize = DEFAULT_GUARD_SIZE;
```

[Visibility]: Internal (不导出) — musl 线程模块内部共享的全局变量，被 `pthread_attr_init`、`pthread_setattr_default_np`、`pthread_getattr_default_np` 等引用。

#### Intent

保存新创建线程的默认守护页大小。初始化为 `DEFAULT_GUARD_SIZE`（8192 = 8KB），可通过 `pthread_setattr_default_np` 增大（不可减小），始终被限制在 `DEFAULT_GUARD_MAX`（1MB）以内。

#### 前置条件

无（初始化时由编译期常量赋值）。

#### 后置条件

- 初始值为 `8192`
- 后续修改需通过 `__inhibit_ptc` / `__release_ptc` 保护
- 值始终 >= 初始值，且 <= `DEFAULT_GUARD_MAX`

#### 不变量

- 任何 `__default_guardsize` 修改必须在持有 PTC 锁时进行
- 值单调不减

---

#### 依赖

- `DEFAULT_STACK_SIZE` — 宏，定义于 `pthread_impl.h`，值为 `131072`
- `DEFAULT_GUARD_SIZE` — 宏，定义于 `pthread_impl.h`，值为 `8192`
- `pthread_impl.h` — 内部头文件，声明了 `extern hidden unsigned __default_stacksize; extern hidden unsigned __default_guardsize;`
