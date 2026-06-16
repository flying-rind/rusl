# __tls_get_addr.c 规约

> musl libc 内部 TLS（线程局部存储）变量地址解析函数。给定 TLS 模块偏移描述符，从当前线程的 DTV（Dynamic Thread Vector）中查找对应模块的基址，加上变量偏移量，返回变量的实际虚拟地址。

---

## 依赖图

```
__tls_get_addr
  ├─> __pthread_self()                    (see pthread_impl.h — 获取当前线程)
  └─> self->dtv[v[0]] + v[1]             (DTV 查找 + 偏移加法)
```

---

## 类型定义

### tls_mod_off_t

```c
#ifndef tls_mod_off_t
#define tls_mod_off_t size_t
#endif
```

[Visibility]: Internal (不导出) — 定义于 `pthread_impl.h`，默认为 `size_t`

TLS 模块偏移描述符的元素类型。`v` 是 `tls_mod_off_t *`，其中：
- `v[0]` 为 TLS 模块 ID（在 DTV 中的索引）
- `v[1]` 为变量在该模块内的偏移量

---

## 函数规约

### 1. __tls_get_addr

```c
void *__tls_get_addr(tls_mod_off_t *v);
```

[Visibility]: Internal (不导出) — 被 `pthread_impl.h` 声明为 hidden，仅 musl 内部使用。通常由 TLS 描述符重定位 (`TLSDESC`) 或 GNU2 TLS 模型的 `__tls_get_addr` 调用。

#### Intent

在运行时解析 TLS 变量的地址。编译器生成的重定位代码在访问动态 TLS 模块中的变量时调用此函数。通过当前线程的 DTV 数组找到目标模块的 TLS 块基址，加上变量的模块内偏移量，返回变量的实际虚拟地址。

#### 前置条件

- `v` 非空，指向至少 2 个 `tls_mod_off_t` 元素的数组
- `v[0]` 为有效的 TLS 模块 ID（在 DTV 范围内）
- 当前线程的 DTV 已初始化（该模块的 TLS 块已分配）
- `v[1]` 为有效的模块内偏移量

#### 后置条件

- 返回值为 `self->dtv[v[0]] + v[1]`
  - `self->dtv[v[0]]` 是 TLS 模块 `v[0]` 在当前线程中的 TLS 块基址
  - `+ v[1]` 加上变量在该模块内的偏移
- 返回值指向当前线程中对应 TLS 变量的可读写位置

#### 系统算法

```
__tls_get_addr(v):
  1. self = __pthread_self()          // 获取当前线程的 pthread 结构体
  2. return (void *)(self->dtv[v[0]] + v[1])
      // self->dtv[v[0]]: 模块 v[0] 的 TLS 基址
      // v[1]: 变量在模块内的偏移
```

#### 不变量

- DTV 索引 `v[0]` 在所有线程中相同（模块 ID 是全局的），但 DTV 值 `self->dtv[v[0]]` 因线程而异（每个线程有自己 TLS 块的副本）
- 此函数不分配内存——TLS 块已在模块加载或线程创建时分配

#### 依赖

- `__pthread_self()` — 获取当前线程的 `pthread_t`（见 `pthread_impl.h`）
- `struct pthread` — 线程结构体，包含 `dtv` 字段（见 `pthread_impl.h`）
- `tls_mod_off_t` — TLS 模块偏移类型（见 `pthread_impl.h`）
