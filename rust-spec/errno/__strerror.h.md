# __strerror.h — Rust 接口归约

> rusl 内部错误消息数据定义模块。将所有标准 errno 错误码映射为人可读的错误描述字符串。在 C 实现中通过 `E(n, s)` 宏协议经 `#define E` + `#include` + `#undef E` 三次包含展开，Rust 中直接声明常量数组以替代宏技巧。

---

## 原始 C 接口

```c
// 宏协议：E(n, s) — 错误码 n 到错误描述字符串 s 的映射
// 被 strerror.c 包含三次：
//   第 1 次 (#define E(n, s) char str##n[sizeof(s)];) → 结构体成员声明
//   第 2 次 (#define E(n, s) s,)                       → 结构体初始化列表
//   第 3 次 (#define E(n, s) [n] = offsetof(..., str##n),) → 偏移量索引表
E(0,            "No error information")
// ... 约 60+ 条 E(n, s) 映射，涵盖 C 标准错误、POSIX 基础错误、
//     文件系统错误、网络错误、线程同步错误、Linux 扩展错误
```

[Visibility]: Internal — musl 内部宏数据定义，仅在 `strerror.c` 中通过 `#define`-`#include`-`#undef` 模式多次包含使用

---

## Rust 内部数据接口

```rust
use core::ffi::c_char;

/// 错误消息原始字节表
///
/// 所有错误消息字符串以 NUL 字节 (`\0`) 结尾，按条目的定义顺序拼接为连续字节数组。
/// 第一条消息（偏移 0）固定为 `"No error information\0"`，作为所有未定义错误码的统一回退消息。
///
/// 表中共约 60+ 条消息，总长度约数百字节，由 `__strerror_l` 在运行时通过偏移量定位。
pub(crate) const ERRMSG_DATA: &[u8];

/// 错误码到消息偏移量的索引表
///
/// `ERRMSG_IDX[e]` 给出错误码 `e`（整数值）对应消息在 `ERRMSG_DATA` 中的起始字节偏移。
/// 数组长度覆盖平台上最大的 errno 值（含 MIPS 兼容的 EDQUOT_ORIG=1133），因此长度至少为 1134。
/// 对于未显式定义的错误码，`ERRMSG_IDX[e] == 0`，指向回退消息 `"No error information"`。
///
/// 每个条目为 `u16`（2 字节），总大小约为 `MAX_ERRNO * 2` 字节。
pub(crate) const ERRMSG_IDX: [u16; N];
```

[Visibility]: Internal — `pub(crate)` 可见性，仅在 `rusl` crate 内部供 `__strerror_l` 使用

---

## 意图

定义从 errno 错误码到人可读错误描述字符串的编译期映射数据。

在 C 实现中，`__strerror.h` 本身只定义 `E(n, s)` 宏条目，不产生实际代码。`strerror.c` 通过三次不同 `#define` 加 `#include` 的模式，将同一份数据分三次展开为：

1. 结构体 `errmsgstr_t` 的成员声明（每成员一个定长字符数组）
2. 结构体 `errmsgstr` 的静态初始化列表
3. 索引表 `errmsgidx`（从 errno 值到结构体内偏移量的映射）

Rust 不需要此类宏技巧。直接声明常量字节数组 `ERRMSG_DATA` 和索引表 `ERRMSG_IDX`，由编译器保证内存布局的正确性。`__strerror_l` 函数在运行时通过 `ERRMSG_IDX[e]` 获取偏移量，再以 `&ERRMSG_DATA[offset..]` 定位 NUL 结尾的消息字节串，最终返回 `*const c_char` 供 FFI 调用。

## 数据不变量

- **静态只读**: `ERRMSG_DATA` 和 `ERRMSG_IDX` 均为编译期常量，存储在只读数据段（`.rodata`），程序生命周期内有效。
- **回退保证**: `ERRMSG_IDX[0] == 0`，且 `ERRMSG_DATA[0..]` 起始为 `"No error information\0"`。任何未定义的错误码均映射到此回退消息。
- **NUL 结尾**: 每条消息均以 `\0` 结尾，可直接作为 C 字符串指针返回。
- **紧凑存储**: 消息之间无填充，由偏移量精确定位起始地址。

## 依赖

无。本模块为纯数据定义，不依赖任何外部函数、类型或宏（仅依赖 `core` 基础类型）。

---

/* Rely */
[RELY]
Predefined Structures/Functions:
  (none)

Predefined Macros/Traits:
  (none)

[GUARANTEE]
Internal Interface:
  pub(crate) const ERRMSG_DATA: &[u8];     // 错误消息拼接字节数组
  pub(crate) const ERRMSG_IDX: [u16; N];   // errno → 偏移量索引表