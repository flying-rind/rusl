# ferror.c 规约

> musl libc 文件流错误状态查询实现。提供 `ferror` 和 POSIX 免锁扩展 `ferror_unlocked`。

---

## 依赖图

```
ferror
  ├─> FLOCK                 (see stdio_impl.h / __lockfile.c spec)
  │     └─> __lockfile      (see __lockfile.c spec)
  └─> FUNLOCK               (see stdio_impl.h / __lockfile.c spec)
        └─> __unlockfile    (see __lockfile.c spec)

ferror_unlocked = weak_alias(ferror)
_IO_ferror_unlocked = weak_alias(ferror)
```

---

## 数据结构分析

`ferror` 宏定义（`stdio_impl.h` 第 98 行）：
```c
#define ferror(f) ((f)->flags & F_ERR)
```

此宏不经加锁直接检查标志位。而函数版 `ferror()` 的实现通过 `#undef ferror` 取消宏定义，提供加锁安全的版本。

---

## 函数规约

### 1. ferror

```c
int ferror(FILE *f);
```

[Visibility]: User — 标准 C 库函数（ISO C），声明于 `<stdio.h>`。用户程序可直接调用。

#### Intent

测试文件流的错误指示符。`F_ERR` 标志由各类 I/O 操作在发生错误时设置，可通过 `clearerr` 或 `rewind` 清除。`ferror` 宏（`stdio_impl.h` 中定义）直接读取 `f->flags & F_ERR`，而此函数版本提供线程安全的加锁访问。

#### 前置条件

- `f`: 非 NULL 的 `FILE*`

#### 后置条件

- `FLOCK(f)` 获取锁后读取 `f->flags`，`FUNLOCK(f)` 后释放锁
- 若 `f->flags` 中 `F_ERR` 标志位被设置（值 `32`），返回非零值（`1`）
- 若 `F_ERR` 未被设置，返回 `0`
- 使用 `!!` 双否定将位掩码结果规范化为 `0` 或 `1`

#### 系统算法

```
ferror(f):
  FLOCK(f)
  ret = !!(f->flags & F_ERR)   // 原子读取 ERR 标志
  FUNLOCK(f)
  return ret
```

#### 不变量

- 仅读取 `f->flags`，不修改任何状态

#### 依赖

- `FLOCK` / `FUNLOCK` — 流锁定/解锁宏（`stdio_impl.h`）
- `F_ERR` — 流错误标志位（`stdio_impl.h`，值 `32`）

---

### 2. ferror_unlocked (weak_alias)

```c
weak_alias(ferror, ferror_unlocked);
```

[Visibility]: User — POSIX 扩展函数，声明于 `<stdio.h>`（需 `_POSIX_C_SOURCE >= 200112L`）。

### 3. \_IO_ferror_unlocked (weak_alias)

```c
weak_alias(ferror, _IO_ferror_unlocked);
```

[Visibility]: Internal — glibc 兼容别名，不直接对用户暴露。供需要 `_IO_*` 符号的旧代码使用。

- **Intention**: 两个弱别名共享同一实现。前置/后置条件完全等同于 `ferror`。
