# acct.c 规约

> musl libc GNU 扩展进程记账系统调用封装。`acct` 在 `<unistd.h>` 中声明（`_GNU_SOURCE`）。

---

## 函数规约

### acct

```c
int acct(const char *filename);
```

[Visibility]: User — `<unistd.h>` GNU 扩展函数，用户程序可直接调用

#### Intent

启用或禁用进程记账。若 `filename` 非 NULL，内核开始将进程记账信息写入指定文件；若 `filename` 为 NULL，禁用记账。需要 `CAP_SYS_PACCT` 权限。

#### 前置条件

- `filename`: 记账文件路径（NULL 禁用记账）
- 需要特权权限

#### 后置条件

- **Case 1 成功**: 记账启用/禁用，返回 0
- **Case 2 错误**: 返回 -1，设置 errno（`EPERM`、`EACCES`、`ENOSYS` 等）

#### 系统算法

```
acct(filename):
  return syscall(SYS_acct, filename)
```

#### 依赖

- `SYS_acct` — Linux 内核系统调用
