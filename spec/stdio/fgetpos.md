# fgetpos.c 规约

> musl libc 文件位置获取实现（ISO C 标准接口）。将与流关联的文件位置指示符的当前值存入 `fpos_t` 对象。

---

## 依赖图

```
fgetpos
  └─> __ftello                   (see ftell.c spec)
```

---

## 数据结构分析

### `fpos_t` 类型

- 在 musl 中 `fpos_t` 定义为 `long long`（足够容纳 `off_t`）
- `fgetpos` 通过 `*(long long *)pos = off` 写入，假定 `fpos_t` 与 `long long` 内存布局兼容
- 写入的值是 `__ftello` 返回的 `off_t` 类型值

---

## 函数规约

### 1. fgetpos

```c
int fgetpos(FILE *restrict f, fpos_t *restrict pos);
```

[Visibility]: User — 标准 C 库函数（ISO C），声明于 `<stdio.h>`。用户程序可直接调用。

#### Intent

获取文件流的当前逻辑位置并存入用户提供的 `fpos_t` 对象。该值后续可通过 `fsetpos` 恢复到同一位置。相比 `ftell`/`fseek`，`fgetpos`/`fsetpos` 使用不透明类型 `fpos_t` 存储位置，可涵盖任意大的文件偏移。

#### 前置条件

- `f`: 非 NULL 的 `FILE*`
- `pos`: 非 NULL 的 `fpos_t*`，指向有效的 `fpos_t` 存储空间
- `f` 的底层定位操作可用（`f->seek` 函数指针有效）

#### 后置条件

**Case 1: 成功**
- `*pos` 被写入代表当前逻辑文件位置的 `off_t` 值（以 `long long` 形式存储）
- 返回 `0`

**Case 2: 失败 — 无法获取位置**
- `__ftello(f)` 返回 `< 0`
- `*pos` 未被修改
- 返回 `-1`
- errno 由 `__ftello` 设置

**Case 3: 失败 — 位置超出 long long 范围**（理论上）

- `__ftello` 可能设置 `EOVERFLOW`（当被 `ftell` 的路径触发时），但 `fgetpos` 不使用 `ftell` 的溢出检查路径
- 注：musl 实现中 `fgetpos` 直接调用 `__ftello` 而非 `ftell`，不进行 `LONG_MAX` 溢出检查。若 `off_t` 为 64 位且文件偏移超出 `long long` 范围（> 9EB），可能存在截断问题，但这在 musl 目标平台（`off_t` ≤ `long long`）上不会发生

#### 系统算法

```
fgetpos(f, pos):
  off = __ftello(f)               // 获取当前 off_t 位置
  if off < 0:                     // 定位失败
    return -1
  *(long long *)pos = off         // 将 off_t 存入 fpos_t
  return 0
```

#### 不变量

- `*pos` 在失败时不被修改（原子性：失败不产生副作用）
- `fpos_t` 中存储的值对应于 `__ftello` 的 `off_t` 返回值（未做变换）

#### 依赖

- `__ftello` — 加锁位置查询（定义于 `src/stdio/ftell.c`，见 `ftell.c` spec）
