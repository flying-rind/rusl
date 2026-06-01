# procfdname.rs 规约 (Rust)

> **来源 C spec**: `musl/src/internal/spec/procfdname.md`
> **对应源文件**: `musl/src/internal/procfdname.c`
> **复杂度层级**: Level 1 — 纯算术实现，无内部/外部函数调用

---

## 依赖图

```
procfdname
  └── (无内部或外部函数调用 — 纯算术实现)
```

该函数是纯自包含实现，不调用任何其他函数，仅依赖 Rust 内建类型和操作符。

---

## 概述

`procfdname` 是 rusl 内部辅助函数，用于构造 Linux procfs 文件描述符路径字符串 `/proc/self/fd/N`。被 `fstat`、`fchdir` 等通过 `/proc` 实现 fd 操作的模块使用。

---

## 函数声明

### `procfdname(buf: &mut [u8], fd: c_uint)`

```rust
// Rust 签名
pub(crate) fn procfdname(buf: &mut [u8], fd: core::ffi::c_uint)
```

[Visibility]: Internal — rusl 内部辅助函数，用于构造 `/proc/self/fd/` 路径字符串。POSIX/C 标准未定义。

**意图 (Intent)**：
在不使用 `snprintf` 或字符串反转的情况下，高效构造 `/proc/self/fd/N` 路径。

---

### 前置条件

1. `buf.len() >= 15 + 3 * core::mem::size_of::<c_int>()`（对于 32 位 `c_int` 为 27 字节）
2. `fd` 为有效的文件描述符编号（非负整数）或 `AT_FDCWD` 特殊值

### 后置条件

**Case 1 — fd == 0**:
- `buf[..15]` 内容为 `"/proc/self/fd/0"`（NUL 终止）
- 该路径总长 = 15 字符（前缀 14 + '0' + NUL）

**Case 2 — fd > 0**:
- `buf` 前缀内容为 `"/proc/self/fd/N"`，其中 `N` 是 `fd` 的十进制字符串表示（NUL 终止）
- 无前导零
- 路径长度为 `15 + floor(log10(fd)) + 1`（不含 NUL，含 NUL 则 +1）

**Case 3 — 任意 fd 值**:
- `buf` 中的路径字符串始终以 NUL 字符终止
- 不修改除 `buf` 外的任何全局或局部状态

### 系统算法（System Algorithm）

该函数使用 **两遍扫描** 策略避免反转或临时缓冲区：

1. **前缀复制**: 将字节字面量 `b"/proc/self/fd/"` 逐字节拷贝到 `buf`，得到前缀 `/proc/self/fd/`，索引停在前缀末尾处（14）

2. **特判 fd == 0**: 直接写入字节 `b'0'` 和 NUL 终止符，返回

3. **第一遍 — 测量数字位数**: 用循环 `while j > 0 { j /= 10; i += 1; }` 计算 `fd` 的十进制位数并将索引 `i` 推进到最终 NUL 位置

4. **NUL 预置**: `buf[i] = b'\0'` 在最终 NUL 所在位置预先写入终止符

5. **第二遍 — 反向填充数字**: 用循环从右向左逐位填充数字字符，自然获得正确的数字顺序（利用 `fd % 10` 取最低位、`i -= 1` 向前移动写入位置的配合）

**意图（Intent）**:
该实现在 O(log10(fd)) 时间内完成，不使用除法反转、字符串反转、临时数组或格式化函数。两遍扫描的技巧利用了 `%10` 取最低位和反向填充的自然配合。

### 不变量（Invariants）

1. 循环不变量（第一遍）: 循环次数等于 `fd` 的十进制位数
2. 循环不变量（第二遍）: 在每轮开始前，`buf[i]` 是上一轮设置的 NUL（于第一轮）或已填充的数字字符
3. 路径前缀 `/proc/self/fd/` 占 14 个字节（不含 NUL），`buf[0..14]` 在整个函数执行过程中不被修改
4. `buf` 输出总是有效的 NUL 终止 C 字符串

---

## Rust 设计要点

- 输入缓冲区使用 `&mut [u8]` 替代 C 裸指针，提供边界安全
- 前缀字节序列使用 `b"/proc/self/fd/"` 字节字面量，避免字符串编码开销
- 数字字符生成使用 `b'0' + (fd % 10) as u8` 的字节运算
- 函数内部不使用任何 `unsafe`：所有索引操作均在已验证的范围内

---

## 跨文件依赖

| 依赖项 | 来源 | 处理方式 |
|--------|------|----------|
| `core::ffi::c_uint` | Rust core 库 | 无需追踪 |
| `core::mem::size_of` | Rust core 库 | 无需追踪 |
| 字节字面量 `b"/proc/self/fd/"` | 编译器内建 | 跳过 |
| 算术操作符 | Rust 语言内建 | 跳过 |

---

## RELY / GUARANTEE

```
[RELY]
（无外部依赖 — 纯算术实现）
Rust Core 内建类型:
  core::ffi::c_uint                // 依赖1: C 兼容的 unsigned int 类型

[GUARANTEE]
pub(crate) 接口:
  fn procfdname(buf: &mut [u8], fd: c_uint)   // 构造 /proc/self/fd/N 路径
```