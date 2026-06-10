# \_\_stdio_exit.c 规约

> musl libc `__stdio_exit` 实现 — 程序退出时的 stdio 清理函数。遍历所有打开的 FILE 流，刷新未写入的缓冲数据，并关闭所有文件描述符。

---

## 依赖图

```
__stdio_exit (Internal)
  ├── __ofl_lock (see ofl.c spec)
  ├── close_file (static, 文件内部)
  │     ├── FFINALLOCK (锁宏, 定义于 stdio_impl.h)
  │     │     └── __lockfile (see __lockfile.c spec)
  │     ├── f->write (FILE 函数指针)
  │     ├── f->seek (FILE 函数指针)
  │     └── (直接访问 f->wpos, f->wbase, f->rpos, f->rend)
  └── __stdin_used / __stdout_used / __stderr_used (weak alias, 哨兵变量)
```

---

## 内部静态函数规约

### 1. close_file (static)

```c
static void close_file(FILE *f);
```

[Visibility]: Internal — `static` 函数，仅文件内可见

#### Intent

安全关闭单个 FILE 流。在关闭前，先刷新未写入的缓冲数据（若 `wpos != wbase`），再修正文件偏移量以补偿已读取但未消费的缓冲区数据。

#### 前置条件

- `f`: 可能为 NULL 的 FILE 指针

#### 后置条件

- Case 1 `f == NULL`:
  - 无操作，直接返回

- Case 2 `f != NULL`:
  - 获取 FILE 的完全锁 `FFINALLOCK(f)`
  - 若 `f->wpos != f->wbase`（写缓冲区有待刷新数据）: 调用 `f->write(f, 0, 0)` 刷新
  - 若 `f->rpos != f->rend`（读缓冲区已被读取但未消费完）: 调用 `f->seek(f, f->rpos - f->rend, SEEK_CUR)` 回退文件位置以补偿多读的数据
  - 不调用 `f->close`（退出时无需逐流关闭，OS 会在进程结束时回收 fd）

#### 系统算法

```
close_file(f):
  1. 若 f == NULL，返回
  2. FFINALLOCK(f) — 完全锁
  3. 若 f->wpos != f->wbase:
       调用 f->write(f, 0, 0) — 刷新写缓冲区
  4. 若 f->rpos != f->rend:
       调用 f->seek(f, f->rpos - f->rend, SEEK_CUR) — 回退文件偏移
  5. (不调用 f->close — 退出时 OS 自动回收)
```

---

## 对外导出函数规约

### 2. \_\_stdio_exit

```c
void __stdio_exit(void);
```

[Visibility]: Internal — `hidden` 可见性，供 `atexit` 回调或 `_Exit` 路径调用，不直接对用户暴露

#### Intent

程序退出时清理所有 stdio 流的入口函数。由 musl 的 exit 路径（`exit()` / `_Exit()`）自动调用，确保所有缓冲数据被刷新到文件系统。

**两层遍历**：
1. 先遍历全局打开文件链表（`ofl_head`）中的显式打开流
2. 再处理 `__stdin_used`、`__stdout_used`、`__stderr_used` 三个标准流

#### 前置条件

- 程序即将退出
- `__ofl_lock()` 返回的全局文件链表可能包含已打开的 FILE 对象
- `__stdin_used`、`__stdout_used`、`__stderr_used` 可能指向有效 FILE 对象，也可能为 NULL（如果链接时未引用任何 stdio 函数，它们通过弱别名指向 NULL）

#### 后置条件

- 全局文件链表中的所有 FILE 流已被刷新（写缓冲区数据已写出）
- `__stdin_used`、`__stdout_used`、`__stderr_used` 指向的流已被刷新（若非 NULL）
- 所有流的文件偏移量已调整以反映实际读/写位置

#### 系统算法

```
__stdio_exit():
  1. 遍历全局打开文件链表:
     for f = *__ofl_lock(); f != NULL; f = f->next:
         close_file(f)
  2. 处理三个标准流:
     close_file(__stdin_used)
     close_file(__stdout_used)
     close_file(__stderr_used)
```

---

### 3. \_\_stdio_exit_needed (weak_alias)

```c
weak_alias(__stdio_exit, __stdio_exit_needed);
```

[Visibility]: Internal — 弱别名，供 musl 内部链接器引用链使用

#### Intent

`__stdio_exit` 的弱别名。musl 的退出路径通过 `__stdio_exit_needed` 符号引用来判断是否需要调用 stdio 清理。若程序中未链接任何 stdio 函数，`__stdio_exit_needed` 不会被定义，exit 路径将跳过 stdio 清理逻辑。

---

### 4. 弱别名哨兵变量

```c
static FILE *volatile dummy_file = 0;
weak_alias(dummy_file, __stdin_used);
weak_alias(dummy_file, __stdout_used);
weak_alias(dummy_file, __stderr_used);
```

[Visibility]: Internal — 弱别名定义，仅当链接时未引用 stdin/stdout/stderr 相关符号时生效

#### Intent

若程序未链接任何引用 stdin/stdout/stderr 的代码（即 `__stdin_used`、`__stdout_used`、`__stderr_used` 的强定义未出现在链接中），则通过弱别名将它们替换为 NULL（`dummy_file`）。这样 `__stdio_exit` 在访问它们时，`close_file(NULL)` 会安全地无操作返回。

---

## 不变量

- **弱别名兜底**：若 stdin/stdout/stderr 未被实际引用，`__std*_used` 变量为 NULL，`close_file` 会安全跳过
- **缓冲区一致性**：退出时，每个流的写缓冲区必须刷新（`wpos` 回到 `wbase`），读缓冲区的文件偏移量必须修正
- **退出安全**：`__stdio_exit` 可以多次调用而不会崩溃（第二次遍历链表可能为空或含已处理流）

## 依赖

- `__ofl_lock()` — 获取全局打开文件链表头指针（定义于 `src/stdio/ofl.c`）
- `__lockfile(FILE *)` — FILE 完全锁（定义于 `src/stdio/__lockfile.c`）
- `f->write(FILE *, const unsigned char *, size_t)` — 流写函数指针
- `f->seek(FILE *, off_t, int)` — 流定位函数指针

---

## 常量引用

| 常量 | 值 | 来源 | 说明 |
|------|-----|------|------|
| `SEEK_CUR` | 1 | `<stdio.h>` | 相对于当前位置的文件定位 |
