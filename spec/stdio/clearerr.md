# clearerr.c 规约

> musl libc 文件流错误状态清除实现。提供 `clearerr` 和 POSIX 免锁扩展 `clearerr_unlocked`。

---

## 依赖图

```
clearerr
  ├─> FLOCK                 (see stdio_impl.h / __lockfile.c spec)
  │     └─> __lockfile      (see __lockfile.c spec)
  └─> FUNLOCK               (see stdio_impl.h / __lockfile.c spec)
        └─> __unlockfile    (see __lockfile.c spec)

clearerr_unlocked = weak_alias(clearerr)
```

---

## 数据结构分析

`clearerr` 清除的标志位：

| 标志 | 值 | 含义 |
|------|-----|------|
| `F_EOF` | `16` | 文件结束指示符 |
| `F_ERR` | `32` | 错误指示符 |

---

## 函数规约

### 1. clearerr

```c
void clearerr(FILE *f);
```

[Visibility]: User — 标准 C 库函数（ISO C），声明于 `<stdio.h>`。用户程序可直接调用。

#### Intent

清除文件流的文件结束指示符（`F_EOF`）和错误指示符（`F_ERR`）。调用后 `feof(f)` 和 `ferror(f)` 均返回 `0`，允许在出错后重试 I/O 操作。

典型使用场景：
1. 在 `fread` 返回 `0` 后区分 EOF 和错误，若为 EOF 则先 `clearerr` 再尝试其他操作
2. I/O 操作失败后，调用 `clearerr` 重置错误状态以便后续操作
3. 在不可定位文件（如管道）上发生 I/O 错误后重置

#### 前置条件

- `f`: 非 NULL 的 `FILE*`

#### 后置条件

- `FLOCK(f)` 获取锁，`FUNLOCK(f)` 释放锁
- `f->flags` 中 `F_EOF` 和 `F_ERR` 位被清除（`f->flags &= ~(F_EOF | F_ERR)`）
- 其他标志位（`F_NOWR`、`F_NORD`、`F_APP`、`F_SVB`、`F_PERM`）保持不变
- 无返回值

#### 系统算法

```
clearerr(f):
  FLOCK(f)
  f->flags &= ~(F_EOF | F_ERR)   // 清除 EOF 和 ERR 标志
  FUNLOCK(f)
```

#### 不变量

- 仅修改 `f->flags` 中的 `F_EOF` 和 `F_ERR` 位，不改变其他任何字段
- 操作在锁保护下原子执行

#### 依赖

- `FLOCK` / `FUNLOCK` — 流锁定/解锁宏（`stdio_impl.h`）
- `F_EOF` / `F_ERR` — 文件结束/错误标志位（`stdio_impl.h`，值 `16` / `32`）

---

### 2. clearerr_unlocked (weak_alias)

```c
weak_alias(clearerr, clearerr_unlocked);
```

[Visibility]: User — POSIX 扩展函数，声明于 `<stdio.h>`（需 `_POSIX_C_SOURCE >= 200112L`）。

- **Intention**: 与 `clearerr` 共享同一实现。POSIX 标准规定 `clearerr_unlocked` 是不加锁版本，但 musl 中两者实现相同（均加锁）。前置/后置条件完全等同于 `clearerr`。
