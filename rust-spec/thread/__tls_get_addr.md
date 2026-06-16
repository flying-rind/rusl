# __tls_get_addr — Rust 接口归约

> rusl 内部 TLS（线程局部存储）变量地址解析函数。给定 TLS 模块偏移描述符，从当前线程的 DTV（Dynamic Thread Vector）中查找对应模块的基址，加上变量偏移量，返回变量的实际虚拟地址。

## 原始 C 接口

```c
void *__tls_get_addr(tls_mod_off_t *v);
```

[Visibility]: Internal — 被 `pthread_impl.h` 声明为 hidden，仅 musl/rusl 内部使用。通常由 TLS 描述符重定位 (`TLSDESC`) 或 GNU2 TLS 模型的运行时解析器调用。

---

## Rust 外部 ABI 接口

```rust
pub extern "C" fn __tls_get_addr(v: *mut usize) -> *mut core::ffi::c_void;
```

> `tls_mod_off_t` 默认为 `size_t`，在 64 位平台上等同于 `usize`。`v` 指向至少 2 个元素的数组：`v[0]` 为模块 ID，`v[1]` 为模块内偏移。

---

## 依赖图

```
__tls_get_addr
  ├─> __pthread_self()                    (外部 — 获取当前线程结构体)
  └─> self.dtv[v[0]] + v[1]              (DTV 查找 + 偏移加法)
```

其中：
- `self` 是当前线程的 `pthread` 结构体
- `self.dtv` 是 Dynamic Thread Vector 数组，`dtv[module_id]` 返回该 TLS 模块在当前线程中的基址

---

## 函数规约

### 1. __tls_get_addr

```rust
pub extern "C" fn __tls_get_addr(v: *mut usize) -> *mut core::ffi::c_void;
```

#### Intent

在运行时解析 TLS 变量的地址。编译器生成的重定位代码在访问动态 TLS 模块中的变量时调用此函数。通过当前线程的 DTV 数组找到目标模块的 TLS 块基址，加上变量的模块内偏移量，返回变量的实际虚拟地址。

Rust 实现中，`v` 指针被解释为 `&[usize; 2]` 切片以安全地访问 `v[0]` 和 `v[1]`。

#### 前置条件

- `v` 非空，指向至少 2 个 `usize` 元素的数组
- `v[0]` 为有效的 TLS 模块 ID（在 DTV 范围内）
- 当前线程的 DTV 已初始化（该模块的 TLS 块已分配）
- `v[1]` 为有效的模块内偏移量

#### 后置条件

- 返回值为 `self.dtv[v[0]] + v[1]`
  - `self.dtv[v[0]]` 是 TLS 模块 `v[0]` 在当前线程中的 TLS 块基址
  - `+ v[1]` 加上变量在该模块内的偏移
- 返回值指向当前线程中对应 TLS 变量的可读写位置

#### 系统算法

```
__tls_get_addr(v):
  1. // 安全地将 v 解释为切片以访问两个元素
  2. let module_id = unsafe { *v };        // v[0]: 模块 ID
  3. let offset = unsafe { *v.add(1) };   // v[1]: 模块内偏移
  4. let self = __pthread_self();          // 获取当前线程的 pthread 结构体
  5. let base = self.dtv[module_id];       // 查找模块基址
  6. (base as usize + offset) as *mut c_void  // 返回变量地址
```

#### 不变量

- DTV 索引 `v[0]` 在所有线程中相同（模块 ID 是全局的），但 DTV 值 `self.dtv[v[0]]` 因线程而异（每个线程有自己 TLS 块的副本）
- 此函数不分配内存 -- TLS 块已在模块加载或线程创建时分配
- 返回的指针在目标线程的整个生命周期内有效

#### 依赖

- `__pthread_self()` — 获取当前线程的 `pthread_t` 结构体（见 `pthread_impl.h`）
- `dtv` 字段 — 线程的 Dynamic Thread Vector 数组
- `tls_mod_off_t` — 默认为 `usize`

---

## Rust 安全包装（模块内部，不对外暴露）

```rust
/// 安全版本的 TLS 地址解析
/// 返回 Option，在 DTV 未初始化或模块 ID 无效时返回 None
pub(crate) fn tls_get_addr(module_id: usize, offset: usize) -> Option<*mut core::ffi::c_void> {
    let thread = __pthread_self();
    if module_id >= thread.dtv.len() {
        return None;
    }
    let base = thread.dtv[module_id];
    if base.is_null() {
        return None;
    }
    Some(unsafe { base.add(offset) } as *mut core::ffi::c_void)
}
```

---

/* Rely */
[RELY]
Predefined Structures/Functions:
  __pthread_self()                        // 依赖1: 获取当前线程结构体
  struct pthread { dtv: [*mut u8] }       // 依赖2: 线程结构体中的 DTV 数组

[GUARANTEE]
Exported Interface:
  pub extern "C" fn __tls_get_addr(v: *mut usize) -> *mut c_void;
                                         // 本模块保证对外提供与 C ABI 兼容的 __tls_get_addr 符号
Internal Interface:
  pub(crate) fn tls_get_addr(module_id: usize, offset: usize) -> Option<*mut c_void>;
                                         // 安全包装，供 crate 内部使用
