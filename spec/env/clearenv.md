# clearenv.c 规约

> 源码位置: `src/env/clearenv.c`
> 标准归属: GNU 扩展（非 POSIX），需定义 `_GNU_SOURCE` 方可使用

---

## 依赖图

```
clearenv (Public)
  ├── __environ        → 外部模块 (src/env/__environ.c), 不展开
  │     └── weak_alias 到 environ (POSIX 标准全局变量)
  ├── __env_rm_add     → weak_alias, 强定义在 src/env/setenv.c, 弱回退为 dummy
  └── dummy            → static 内部函数 (本文件)
```

**依赖说明**:
- `__environ`: musl 内部的全局 `char **` 指针，存储环境变量数组首地址。未链接 `setenv`/`putenv` 时，其内容由内核/CRT 传递的环境块提供。
- `__env_rm_add`: 通过 `weak_alias(dummy, __env_rm_add)` 创建弱符号。若链接了 `src/env/setenv.c`，则强定义覆盖此弱符号，提供真实的堆内存管理；否则回退到 `dummy` 无操作。此设计使得 `clearenv` 在使用 `setenv`/`putenv` 时可正确释放堆分配字符串，而未使用这些函数时无额外开销。
- `weak_alias`: musl 内部宏 (`src/include/features.h:8`)，生成 `extern __typeof(old) new __attribute__((__weak__, __alias__(#old)))`，创建一个指向 `old` 的弱别名符号。

---

## dummy (内部函数)

```c
static void dummy(char *old, char *new) {}
```

[Visibility]: **Internal (不导出)** — `static` 函数，musl 内部实现细节，POSIX/C 标准未定义。

### 前置条件
- 无。此函数无任何调用前提条件。

### 后置条件
- Case 1 (总是): 函数体为空，无任何副作用。参数 `old` 和 `new` 被忽略，不作任何操作。

### Intent
作为 `__env_rm_add` 弱别名的默认回退桩函数。当程序未链接 `setenv.c`（即未使用 `setenv`/`putenv`）时，`clearenv` 中对 `__env_rm_add` 的调用将解析到此 `dummy` 函数。由于此时环境变量字符串来自内核/CRT 传递的原始内存区域，无需（也不能）通过 `free()` 释放，因此无操作是完全正确的。

---

## weak_alias(dummy, __env_rm_add)

```c
weak_alias(dummy, __env_rm_add);
```

[Visibility]: **Internal (不导出)** — 文件作用域弱符号声明，`__env_rm_add` 对外不可直接调用（是 musl 内部 hidden 接口）。

### 前置条件
- 链接时：若 `src/env/setenv.c` 的目标文件被链接，则其提供 `__env_rm_add` 的强定义，覆盖此弱别名。

### 后置条件
- Case 1 (setenv.o 已链接): `__env_rm_add` 解析为 `src/env/setenv.c` 中的实现，该实现维护内部分配字符串的记录表，在替换或清除时调用 `free()` 释放旧字符串。
- Case 2 (setenv.o 未链接): `__env_rm_add` 解析为 `dummy` 函数，调用时无任何副作用。

### Intent
通过 GNU `weak_alias` 机制实现**可选依赖**：当 `clearenv` 被调用时，若用户曾通过 `setenv`/`putenv` 分配过堆上的环境字符串（由 `__env_rm_add` 的强实现登记），则这些字符串被正确释放；否则直接忽略，因为 `__environ` 中存储的原始环境字符串不可被 `free()` 释放。这是一种无链接开销的零成本抽象。

---

## clearenv (对外导出)

```c
int clearenv(void);
```

[Visibility]: **Public (对外导出)** — GNU 扩展，声明于 `<stdlib.h>`（需定义 `_GNU_SOURCE` 宏）。用户程序可直接调用。

### 前置条件
- `__environ` 指针可为任意值：
  - 指向 `char *` 数组（由操作系统/CRT 传递的环境块），以 `NULL` 终止
  - 为 `NULL`（环境变量已清空）
  - 指向的数组中含有由 `setenv`/`putenv` 通过 `malloc` 分配后通过 `__env_rm_add` 登记的堆上字符串
- 调用无需外部锁，但多线程环境下并发修改 `__environ` 是**未定义行为**（符合 POSIX 关于 environ 的线程安全限制）。

### 后置条件
- **返回值**: 始终返回 `0`（成功）。
- **状态转换**:
  1. `__environ` 被设置为 `NULL`（空指针），环境变量数组被清空。
  2. 遍历旧的 `__environ` 数组，对每个非 `NULL` 条目调用 `__env_rm_add(entry, NULL)`：
     - 若 `__env_rm_add` 为强实现（`setenv.c` 已链接）：若 `entry` 在分配记录表中，将其解除登记并调用 `free(entry)` 释放内存；否则无操作。
     - 若 `__env_rm_add` 为弱实现（`dummy`）：无操作。
- **不变量**: 调用后 `__environ == NULL`，即外部通过 `environ`/`getenv()` 访问将得到空环境。

### 系统算法

```
算法 clearenv:
  输入: 无
  输出: 0（总是成功）

  1. e := __environ          // 保存旧的环境数组指针
  2. __environ := NULL        // 立即清空全局环境指针
  3. 若 e ≠ NULL:
       对于 e 指向的数组中每个非 NULL 元素 s:
         __env_rm_add(s, NULL)  // 通知环境修改，可能释放堆内存
         移动到下一个元素
  4. 返回 0
```

设计要点：
- **先清空后释放**: 先将 `__environ` 置为 `NULL`，再遍历旧数组调用 `__env_rm_add`。这保证了在 `__env_rm_add` 的回调过程中，任何对 `getenv()` 的调用都已看到空环境。
- **弱符号解耦**: 通过 `weak_alias` 避免了 `clearenv` 对 `setenv.c` 的硬链接依赖。这也意味着在不使用 `setenv`/`putenv` 的程序中，调用 `clearenv` 不会引入任何 `malloc`/`free` 相关的代码和开销。

### Intent
清除当前进程的所有环境变量。这是 `unsetenv` 遍历所有键的批量操作等价物，但其实现利用直接操作 `__environ` 指针来避免逐键查找的开销。通过 `__env_rm_add` 弱符号回调，确保由 `setenv`/`putenv` 分配的堆内存被正确回收，防止内存泄漏。调用后，`environ`（即 `__environ` 的别名）为 `NULL`，任何后续的 `getenv()` 调用都返回 `NULL`，`setenv`/`putenv` 将从空环境开始重新建立环境变量表。