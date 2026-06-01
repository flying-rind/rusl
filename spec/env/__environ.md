# __environ.c 规约

## 依赖图

```
__environ (全局变量, 定义) → weak_alias 宏 (features.h, 编译时展开)
                            → environ (POSIX Public 别名)
                            → _environ (Internal 别名)
                            → ___environ (Internal 别名)
                            → unistd.h (外部标准头文件)
```

> 本文件为纯数据定义，不包含任何函数。无内部函数依赖需要递归追踪。

---

## 外部依赖声明

| 依赖项 | 来源 | 是否展开 |
|--------|------|----------|
| `<unistd.h>` | POSIX 标准头文件 | 否 — 外部标准库 |
| `weak_alias` 宏 | `src/include/features.h` (musl 内部) | 仅说明语义，不递归展开 |

### weak_alias 宏语义

```c
#define weak_alias(old, new) \
    extern __typeof(old) new __attribute__((__weak__, __alias__(#old)))
```

该宏为符号 `old` 创建一个名为 `new` 的弱别名：
- `__attribute__((__alias__("old")))` — `new` 与 `old` 共享同一内存地址，即对 `new` 的读写等同于对 `old` 的读写
- `__attribute__((__weak__))` — 若用户程序定义了同名符号，则用户的定义优先，此处定义被覆盖
- 别名之间的关系是**等价**的：读取其中一个即读取所有，写入其中一个即写入所有

---

## __environ (全局变量定义)

```c
char **__environ = 0;
```

[Visibility]: **Public** (间接导出) — `__environ` 是 musl 内部约定的环境变量指针名称，但通过 `extern char **environ;` (`<unistd.h>` 中声明) 和 `weak_alias(__environ, environ)` 暴露给用户程序，因此 `environ` 为 POSIX 标准导出符号，`__environ` 是其内部实现名。

### 类型定义

```c
char **__environ;
```

即 `__environ` 是指向字符串指针数组的指针。该数组的实际格式如下：

```
environ[0] = "HOME=/home/user\0"
environ[1] = "PATH=/usr/bin\0"
environ[2] = "LANG=en_US.UTF-8\0"
...
environ[n] = NULL  (终止哨兵)
```

每个元素的格式为 `"NAME=VALUE"` 的 NUL-terminated C 字符串，数组以 NULL 指针作为终止标记。

### 初始值

```c
char **__environ = 0;  // 即 NULL
```

程序加载时 `__environ` 被初始化为 NULL。实际的栈/堆环境变量数组由程序启动代码在 `main()` 调用之前填入，具体由以下模块负责：

- `src/env/__libc_start_main.c` — 从 `main()` 的第三个参数 `envp` 获取初始环境指针，在调用 `__libc_start_init()` 之前赋值给 `__environ` / `__init_tls` 等初始化路径
- `src/env/__init_tls.c` — 若采用静态 TLS 初始化路径，也可能在此处设置

### 不变量 (Invariants)

1. **终止哨兵不变量**：`__environ` 指向的字符串数组必须以 NULL 指针作为终止标记。即若 `__environ[i]` 为 NULL，则对于所有 `j > i`，`__environ[j]` 亦应视为越界访问。

2. **格式不变量**：每个非 NULL 的 `__environ[i]` 必须是格式为 `"NAME=VALUE"` 的字符串，其中：
   - `NAME` 是非空字符串，仅包含可移植字符集（字母、数字、下划线），不能包含 `'='`
   - `VALUE` 可以是任意字符串（可以为空），以 NUL 终止

3. **无重复-NAME 不变量**（POSIX）：环境变量数组中不应出现同名的条目。若发生，`getenv()` 的行为是返回第一个匹配项（由 musl 的实现决定）。

4. **所有权不变量**：`__environ` 及其中字符串的所有权归属进程。`putenv()` 可能使得其中部分指针指向调用者提供的内存（而非堆分配），此时调用者不得释放该内存直到该条目被覆盖或删除。

### 别名关系

通过 `weak_alias` 宏，以下四个符号**共享同一内存位置**（读/写任一即读写全部）：

| 符号名 | 可见性 | 说明 |
|--------|--------|------|
| `__environ` | Internal (不导出) | musl 内部实现用名，`__` 前缀表示内部符号。**仅 musl 内部代码访问。** |
| `___environ` | Internal (不导出) | musl 内部备用名，`__` 前缀表示内部符号。历史兼容性保留。 |
| `_environ` | Internal (不导出) | musl 内部备用名，`_` 前缀表示内部符号。历史兼容性保留。 |
| `environ` | **Public** — POSIX 标准，`<unistd.h>` 声明 | 用户程序可直接使用 `extern char **environ;` 访问 |

> **注意**: 虽然 `__environ`、`___environ`、`_environ` 通过 `weak_alias` 在技术上也是全局符号，但它们的命名以 `_` 或 `__` 前缀，标准保留供实现使用，用户程序不应直接访问这些名称。用户应统一使用 `environ`。

### 前/后置条件 (关于使用者)

此文件中无函数，仅定义全局变量。以下是该变量的访问契约，供使用者（musl 内部模块及外部程序）参考。

#### 读取 __environ / environ

```
{P} char **e = environ; {Q}
```

- **前置条件**：`__environ` 已被初始化（即 `main()` 已开始执行或等效的启动过程已完成）。在 `main()` 之前（如全局构造函数中），值可能仍为 NULL。
- **后置条件**：
  - Case 1 (正常情况): 返回指向环境字符串数组的指针，数组以 NULL 哨兵终止。调用者可遍历 `environ[i]` 直到 `environ[i] == NULL`。
  - Case 2 (未初始化): 若在程序启动早期阶段读取，可能返回 NULL。调用者应对 NULL 做防护检查。

#### 写入 __environ / environ

```
{P} environ = new_array; {Q}
```

- **前置条件**：`new_array` 必须指向一个以 NULL 终止的 `char *` 数组，或为 NULL。若非 NULL，数组中的每个字符串必须满足 "NAME=VALUE" 格式不变量。
- **后置条件**：
  - `environ` 指向 `new_array`。
  - 旧环境数组本身**不会被释放**：释放旧环境内存是调用者或 `clearenv()` / 相关 API 的责任。
  - 所有通过 `environ` 读取环境变量的操作（如 `getenv()`）立即反映新的环境数组。

> **系统算法**: musl 中 `__environ` 使用简单的全局指针，不采用线程局部存储 (TLS)，即环境变量在进程级别而非线程级别共享。修改 `environ` 会立即影响所有线程的后续环境变量访问。

---

## 相关文件与使用场景

| 模块 | 关系 | 说明 |
|------|------|------|
| `src/env/getenv.c` | 使用者 | 遍历 `__environ` 数组查找指定 NAME |
| `src/env/putenv.c` | 使用者 | 修改 `__environ` 数组（直接替换指针），不重新分配 |
| `src/env/setenv.c` | 使用者 | 修改环境时可能重新分配 `__environ` 数组 |
| `src/env/unsetenv.c` | 使用者 | 从 `__environ` 数组中移除条目 |
| `src/env/clearenv.c` | 使用者 | 将 `__environ` 置为 NULL 或空数组 |
| `src/env/__libc_start_main.c` | 初始化者 | 将 `envp` 参数赋值给 `__environ`，完成初始化 |
| `src/env/__init_tls.c` | 初始化者 | 在 TLS 初始化流程中设置 `__environ` |