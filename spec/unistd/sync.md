# sync.c 规约

> musl libc BSD/XOPEN 扩展同步所有文件系统缓冲区系统调用封装。`sync` 在 `<unistd.h>` 中声明（`_BSD_SOURCE` / `_XOPEN_SOURCE`）。

---

## 函数规约

### sync

```c
void sync(void);
```

[Visibility]: User — `<unistd.h>` BSD/XOPEN 扩展函数，用户程序可直接调用

#### Intent

将所有已修改的文件系统缓冲区和元数据写入磁盘。调用不等待 I/O 完成即返回。通常由系统管理员在关机前调用以确保数据完整性。

#### 前置条件

- 无

#### 后置条件

- 所有脏缓冲区已提交到磁盘 I/O 队列（但可能尚未物理写入），函数返回
- 无返回值

#### 系统算法

```
sync():
  __syscall(SYS_sync)
```

#### 依赖

- `SYS_sync` — Linux 内核系统调用
