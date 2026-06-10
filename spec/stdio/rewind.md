# rewind.c 规约

> musl libc 文件流回绕实现。将文件位置重置到起始并清除错误状态。

---

## 依赖图

```
rewind
  ├─> FLOCK                     (see stdio_impl.h / __lockfile.c spec)
  │     └─> __lockfile          (see __lockfile.c spec)
  ├─> __fseeko_unlocked         (see fseek.c spec)
  │     ├─> f->write            (function pointer)
  │     └─> f->seek             (function pointer, default: __stdio_seek)
  │         └─> __lseek         (see <unistd.h>)
  └─> FUNLOCK                   (see stdio_impl.h / __lockfile.c spec)
        └─> __unlockfile        (see __lockfile.c spec)
```

---

## 函数规约

### 1. rewind

```c
void rewind(FILE *f);
```

[Visibility]: User — 标准 C 库函数（ISO C），声明于 `<stdio.h>`。用户程序可直接调用。

#### Intent

将文件流位置回绕到文件起始，同时清除错误标志 `F_ERR`。相比 `fseek(f, 0, SEEK_SET)`，`rewind` 额外清除了错误状态。

这种行为等价于 `(void)fseek(f, 0, SEEK_SET)` 后再调用 `clearerr(f)` 中的 `f->flags &= ~F_ERR` 部分，但不涉及 `clearerr` 的锁管理——`rewind` 在一次加锁操作中完成所有步骤。

注：`rewind` 不返回值且不清除 `errno`，即使底层 `__fseeko_unlocked` 失败也无法获知错误。

#### 前置条件

- `f`: 非 NULL 的 `FILE*`
- 文件流可定位（文件可 seek）
- 调用方不持有 `f` 的锁

#### 后置条件

- `FLOCK(f)` 获取锁，`FUNLOCK(f)` 释放锁（保证操作的原子性）
- 文件位置指示符被设置为文件起始（通过 `__fseeko_unlocked(f, 0, SEEK_SET)`）
  - 写缓冲区被刷写，读缓冲区被丢弃
  - `F_EOF` 标志被清除（由 `__fseeko_unlocked` 内部完成）
- `f->flags` 中 `F_ERR` 标志被清除
- 不返回错误码——即使底层 seek 失败，函数也不上报

#### 系统算法

```
rewind(f):
  FLOCK(f)
  __fseeko_unlocked(f, 0, SEEK_SET)   // 回绕到起始（丢弃返回值）
  f->flags &= ~F_ERR                   // 清除错误标志
  FUNLOCK(f)
```

#### 不变量

- `F_ERR` 标志在函数返回时一定被清除（即使 seek 失败）
- 操作在锁保护下原子执行（不会与其他线程的 I/O 操作交错）
- 函数无返回值——调用方无法通过返回值或 errno 判断操作是否成功（这是 ISO C 标准行为）

#### 依赖

- `FLOCK` / `FUNLOCK` — 流锁定/解锁宏（`stdio_impl.h`）
- `__fseeko_unlocked` — 不加锁定位（定义于 `src/stdio/fseek.c`，见 `fseek.c` spec）
- `F_ERR` — 流错误标志（`stdio_impl.h`，值 `32`）
