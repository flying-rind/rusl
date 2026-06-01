# unsetenv.c 规约

## 依赖图

```
__environ (外部, see __environ.c)
  └── unsetenv ──> __strchrnul (see strchrnul.c spec)
              ├──> strncmp (外部 POSIX, see strncmp.c spec)
              ├──> __env_rm_add (弱别名, 实际定义在 setenv.c 或退化为 dummy)
              └──> errno / EINVAL (外部标准)

__env_rm_add (弱别名) ──> dummy (默认无操作)
                     └──> __env_rm_add (setenv.c 中的强定义, 覆盖弱别名)
```

---

## 1. `__environ` — 环境变量指针数组（外部符号）

[Visibility]: Internal — musl 内部符号，通过 `weak_alias` 暴露为 POSIX `environ`

**声明所在文件**: `src/env/__environ.c`

```
char **__environ = 0;
weak_alias(__environ, ___environ);
weak_alias(__environ, _environ);
weak_alias(__environ, environ);
```

**语义**: 指向以 NULL 终止的 `"key=value"` 字符串指针数组。初始值为 `NULL`（`= 0`），表示未分配任何环境变量。外部使用者通过 POSIX 标准的 `environ` 访问该符号。

**不变量**:
- 若 `__environ != NULL`，则其指向一个以 NULL 终止的 `char *` 数组
- 数组中每个非 NULL 元素均指向以 `=` 分隔的 `"key=value"` 格式字符串

---

## 2. `dummy` / `__env_rm_add`（弱别名，内部符号）

[Visibility]: Internal — musl 内部辅助函数，不对外导出。`dummy` 为 `static` 函数，`__env_rm_add` 为 `weak_alias` 弱符号，可被其他翻译单元中的强定义覆盖。

```
static void dummy(char *old, char *new) {}
weak_alias(dummy, __env_rm_add);
```

### 意图 (Intent)

`__env_rm_add` 是 musl 环境变量管理模块（`setenv.c`、`putenv.c`、`unsetenv.c`、`clearenv.c`）之间的**跨文件内部通信机制**。其目的是跟踪哪些环境变量字符串需要被释放（即由 `setenv` 通过 `malloc` 分配的字符串）。

该机制的运作方式：
1. 在 `setenv.c` 中定义了 `__env_rm_add` 的**强定义**（覆盖弱别名），负责维护一个已分配字符串的登记表
2. 在 `unsetenv.c`、`putenv.c`、`clearenv.c` 中定义了 `dummy` + `weak_alias(dummy, __env_rm_add)`，作为**默认无操作实现**
3. 链接时：若 `setenv.c` 未参与链接（即程序未使用 `setenv`），则 `__env_rm_add` 解析为无操作的 `dummy`；若 `setenv.c` 参与链接，则其强定义覆盖弱别名，所有调用处实际调用 `setenv.c` 中的实现

### 前置条件

- 未使用 `setenv` 时：无前置条件（`dummy` 为空函数体）
- 使用 `setenv` 时：`old` 指向一个先前由 `malloc` 分配的字符串（待释放），`new` 为替换字符串或 `NULL`

### 后置条件

- 未使用 `setenv` 时：不做任何事（空操作）
- 使用 `setenv` 时：将已分配字符串登记表中 `old` 对应的条目替换为 `new`，并释放 `old`。详见 `src/env/setenv.c` 规约。

---

## 3. `unsetenv` — 删除环境变量（对外导出函数）

[Visibility]: Public — POSIX 标准函数，声明于 `<stdlib.h>`

```
int unsetenv(const char *name);
```

### 意图 (Intent)

从进程环境变量列表中移除指定名称的环境变量。函数对 `__environ` 指针数组执行**原地压缩（in-place compaction）**：遍历数组中的每个条目，将匹配 `"name=value"` 的条目移除（通过 `__env_rm_add` 标记释放），并将不匹配的条目向数组头部移动以填补空隙。该算法在一次遍历中同时完成查找和压缩，时间复杂度 O(n)，空间复杂度 O(1)。

### 前置条件

- `name` 非 NULL（否则 `__strchrnul(name, '=')` 引发未定义行为）
- `name` 非空字符串（`l > 0`），且不包含 `=` 字符（`name[l] == '\0'`）
- `__environ` 若为非 NULL，则为以 NULL 终止的有效 `char *` 数组

### 后置条件

| Case | 条件 | 返回值 | `errno` | `__environ` 状态 |
|------|------|--------|---------|-------------------|
| **Case 1** | `name` 为合法环境变量名（非空、不含 `=`）且 `__environ` 中存在匹配项 | 0 | 不变 | `__environ` 数组被压缩，匹配项被移除，末尾补 NULL |
| **Case 2** | `name` 为合法环境变量名但 `__environ` 中无匹配项 | 0 | 不变 | `__environ` 不变 |
| **Case 3** | `name` 为空字符串（`l == 0`） | -1 | `EINVAL` | `__environ` 不变 |
| **Case 4** | `name` 包含 `=` 字符（`name[l] != '\0'`） | -1 | `EINVAL` | `__environ` 不变 |
| **Case 5** | `__environ == NULL` 且 `name` 合法 | 0 | 不变 | 无操作（循环体不执行） |

### 系统算法 (System Algorithm) — Level 3

函数采用**单趟双指针原地压缩算法**，同时完成匹配扫描和数组压缩：

**算法流程：**

```
输入: name (环境变量名)
1.  计算 l = __strchrnul(name, '=') - name
     // l 为 name 的长度（若 name 中无 '='）或首个 '=' 的索引
2.  验证 name: 若 l == 0 或 name[l] != '\0' (即包含 '=')
       设置 errno = EINVAL, 返回 -1
3.  若 __environ == NULL, 直接返回 0 (无操作)
4.  初始化: e = __environ, eo = __environ
    // e:  读指针 (reader) — 遍历所有条目
    // eo: 写指针 (writer) — 指向下一个保留条目的写入位置
5.  遍历: 对每个 *e (直到 *e == NULL):
    5a. 若 strncmp(name, *e, l) == 0 且 (*e)[l] == '=':
          调用 __env_rm_add(*e, 0)   // 标记旧条目待释放
          // eo 不动（跳过此项，产生间隙）
    5b. 否则（不匹配）:
          若 eo != e: *eo++ = *e     // 向前移动保留条目
          否则:       eo++           // 指针同步前进
    5c. e++ (读指针前进)
6.  若 eo != e (有项被移除): *eo = NULL (重新终止数组)
7.  返回 0
```

**指针状态说明**:

| 状态 | `e`（读指针） | `eo`（写指针） | 含义 |
|------|---------------|----------------|------|
| 初始 | 指向 `__environ[0]` | 指向 `__environ[0]` | eo == e，无间隙 |
| 匹配后 | 前进到下一项 | 停留在当前项 | eo < e，存在间隙，后续保留项需前移 |
| 不匹配+无间隙 | 前进 | 前进 | 同步移动，无需复制（`*=*` 操作省略） |
| 不匹配+有间隙 | 前进 | 复制后前进 | `*eo++ = *e` 将保留项移动到间隙处 |

### 不变量 (Invariants)

**循环不变量**：
- 在每次循环迭代开始前，`eo <= e`（写指针不超过读指针）
- `__environ[0..eo)` 中均为已完成处理的非匹配条目（即不包含名为 `name` 的环境变量）
- `__environ[eo..e)` 中的条目已被丢弃（如未显式清零则为悬空引用，但无碍——后续会被覆盖或位于数组新末尾之后）

**模块级不变量**：
- 若 `unsetenv` 返回 0，`__environ` 中不存在键名为 `name` 的环境变量条目（幂等性保证）

### 写入策略

函数对 `__environ` 数组进行原地修改：
- 匹配项的位置被后续保留项覆盖（通过 `*eo++ = *e`）
- 仅当 `eo != e`（即有项被移除）时才写入新的 NULL 终止符，避免不必要的写入
- 该实现是线程不安全的（符合 POSIX 语义——`setenv`/`unsetenv`/`putenv` 非线程安全）

### 跨文件交互说明

`unsetenv` 调用 `__env_rm_add(*e, 0)` 来通知已分配字符串的管理模块：该环境变量条目（`*e`）即将从 `__environ` 中移除。由于第二个参数为 `0`（NULL），表示没有新的替换字符串。

- **若 setenv.c 已链接**：`__env_rm_add` 的强定义将释放 `*e` 指向的内存（如果是由 `setenv` 分配的），并清理其在登记表中的记录
- **若 setenv.c 未链接**：`__env_rm_add` 解析为 `dummy` 无操作——此时 `*e` 可能指向只读的启动环境字符串，不应释放