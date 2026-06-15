# getlogin_r.c 规约

> musl libc POSIX 登录用户名获取函数（线程安全版）。`getlogin_r` 在 `<unistd.h>` 中声明。

---

## 依赖图

```
getlogin_r (Public)
  ├── getlogin() — 获取 LOGNAME 环境变量值
  ├── strlen(logname) — 计算用户名长度
  └── strcpy(name, logname) — 将用户名拷贝到用户缓冲区
```

---

## 函数规约

### getlogin_r

```c
int getlogin_r(char *name, size_t size);
```

[Visibility]: User — `<unistd.h>` POSIX 标准函数，用户程序可直接调用

#### Intent

获取当前登录用户的名字，写入用户提供的缓冲区 `name`。线程安全版本，使用用户分配的缓冲区而非内部静态变量。

#### 前置条件

- `name`: 指向至少 `size` 字节可用内存的非空指针
- `size`: 缓冲区大小，`> 0`

#### 后置条件

- **Case 1 成功**
  - `name` 缓冲区被写入以 null 结尾的登录用户名
  - 返回 0

- **Case 2 无法获取登录名**
  - 返回 `ENXIO`
  - `name` 内容未定义

- **Case 3 缓冲区太小（无法容纳登录名和 null 终止符）**
  - 返回 `ERANGE`
  - `name` 内容未定义

#### 系统算法

```
getlogin_r(name, size):
  logname = getlogin()                // 1. 获取 LOGNAME 环境变量值
  if logname == NULL:
    return ENXIO                      // 2a. 不存在，返回 ENXIO

  if strlen(logname) >= size:
    return ERANGE                     // 2b. 缓冲区太小（无空间放 null），返回 ERANGE

  strcpy(name, logname)               // 2c. 拷贝用户名到用户缓冲区
  return 0                            // 3. 成功
```

即：
1. 调用 `getlogin()` 获取登录名（环境变量 `LOGNAME` 的值）
2. 若返回 `NULL`（环境变量不存在），返回 `ENXIO` 错误码
3. 检查用户名长度：若 `strlen(logname) >= size`，返回 `ERANGE`（注意：条件是 `>=` 而非 `>`，因为需要额外一个字节存放 null 终止符）
4. 使用 `strcpy` 将用户名拷贝到用户缓冲区
5. 返回 0 表示成功

#### 依赖

- `getlogin` — 非线程安全版登录名获取（`src/unistd/getlogin.c`）
- `strlen` — 字符串长度计算（`src/string/strlen.c`）
- `strcpy` — 字符串拷贝（`src/string/strcpy.c`）
- `ENXIO` — "无此设备或地址" 错误码，此处表示无登录名可用，定义在 `<errno.h>`
- `ERANGE` — 结果超出范围错误码，此处表示缓冲区太小，定义在 `<errno.h>`
