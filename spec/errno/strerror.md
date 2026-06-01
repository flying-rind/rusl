# strerror.c 规约

> musl libc `strerror`/`strerror_l` 实现。将 `errno` 错误码映射为人可读的错误描述字符串，支持 locale 本地化。

---

## 依赖图

```
strerror / strerror_l
  └─> __strerror_l
        └─> LCTRANS  (locale message translation, see locale_impl.h)
```

---

## 数据结构

### 错误消息字符串表

```c
static const struct errmsgstr_t { ... } errmsgstr;
```

[Visibility]: Internal — musl 内部静态只读数据，不对外导出

#### Intent

编译时通过 `__strerror.h` 中的 `E(n, s)` 宏展开，构建包含所有标准 errno 错误消息的紧凑结构体。每个错误码对应一个固定大小的字符数组（刚好容纳对应的消息字符串）。

### 错误消息索引表

```c
static const unsigned short errmsgidx[] = { ... };
```

[Visibility]: Internal — musl 内部静态只读数据，不对外导出

#### Intent

通过 `offsetof` 构建从错误码 `n` 到 `errmsgstr` 中对应字符串偏移量的快速查找表。`errmsgidx[n]` 给出错误码 `n` 对应消息在 `errmsgstr` 中的字节偏移。

### MIPS EDQUOT 兼容处理

```c
#if EDQUOT==1133
#define EDQUOT_ORIG 1133
#undef  EDQUOT
#define EDQUOT 109
#endif
```

[Visibility]: Internal — 编译时预处理，不产生符号

- **Intention**: MIPS 架构历史上将 `EDQUOT` 的值错误地定义为 1133（超出常见范围），musl 内部将其重映射为 109 以便在 16 位索引表中正确寻址。

---

## 函数规约

### 1. \_\_strerror_l

```c
char *__strerror_l(int e, locale_t loc);
```

[Visibility]: Internal — musl 内部 locale-aware 实现

#### Intent

将错误码 `e` 转换为对应的人可读错误描述字符串，并使用 `loc` 指定的 locale 进行消息翻译（若可用）。

#### 前置条件

- `e` 为整数错误码（通常为 `errno` 值）
- `loc` 为有效的 `locale_t` 句柄

#### 后置条件

**Case 1: 已知错误码** (`0 <= e < len(errmsgidx)`):
- 返回指向静态只读错误消息字符串的指针
- 若错误码为 0，返回 `"No error information"`
- 消息经 `LCTRANS` 进行 locale 翻译

**Case 2: 未知错误码** (`e < 0` 或 `e >= len(errmsgidx)`):
- 将 `e` 重置为 0，返回通用消息 `"No error information"`

**Case 3: MIPS EDQUOT 兼容**:
- 若 `e == EDQUOT`（内部重映射值），将其映射为 0
- 若 `e == EDQUOT_ORIG`（原始错误值 1133），将其映射为内部 EDQUOT 值

#### 系统算法

```
__strerror_l(e, loc):
  #ifdef EDQUOT_ORIG:
    if e == EDQUOT:      e = 0
    else if e == EDQUOT_ORIG: e = EDQUOT
  if e >= sizeof(errmsgidx) / sizeof(*errmsgidx):
    e = 0
  s = (char*)&errmsgstr + errmsgidx[e]
  return LCTRANS(s, LC_MESSAGES, loc)
```

#### 不变量

- 返回值始终指向有效的 NUL 结尾字符串
- 返回的指针指向静态存储，调用者不应释放
- 线程安全：不修改任何共享状态

---

### 2. strerror

```c
char *strerror(int e);
```

[Visibility]: Public — ISO C 标准函数，定义于 `<string.h>`

#### Intent

将错误码 `e` 转换为当前 locale 下的错误描述字符串。等价于 `__strerror_l(e, CURRENT_LOCALE)`。

#### 前置条件

- 同 `__strerror_l`

#### 后置条件

- 等同 `__strerror_l(e, CURRENT_LOCALE)`

---

### 3. strerror_l (weak_alias)

```c
weak_alias(__strerror_l, strerror_l);
```

[Visibility]: Public — POSIX.1-2008 扩展，定义于 `<string.h>`

- **Intention**: 通过弱别名将 `__strerror_l` 暴露为 POSIX 标准函数 `strerror_l`。

前置/后置条件及行为：完全等同于 `__strerror_l`。
