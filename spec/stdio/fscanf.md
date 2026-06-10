# fscanf.c 规约

> musl libc 文件流格式化输入函数。是 `vfscanf(f, ...)` 的可变参数包装。

---

## 依赖图

```
fscanf
  └─> vfscanf(f, fmt, ap)  (see vfscanf.c spec)

__isoc99_fscanf (weak_alias)
  └─> fscanf
```

---

## 函数规约

### 1. fscanf

```c
int fscanf(FILE *restrict f, const char *restrict fmt, ...);
```

[Visibility]: User — 通过 `<stdio.h>` 对外导出

#### Intent

从指定的 `FILE` 流 `f` 读取格式化输入。是 `vfscanf` 的可变参数包装器。

#### 前置条件

- `f` 指向有效的 `FILE` 对象，可读取
- `fmt != NULL`，指向有效的格式化字符串
- 可变参数与格式串匹配（指针类型参数必须指向有效位置）

#### 后置条件

- Case 1 成功：返回成功匹配并赋值的输入项数
- Case 2 输入失败（首个转换前到达 EOF）：返回 `EOF`
- `va_list` 在返回前已通过 `va_end` 清理

#### 系统算法

```
fscanf(f, fmt, ...):
  1. va_start(ap, fmt) 初始化可变参数列表
  2. ret = vfscanf(f, fmt, ap) 委托核心引擎
  3. va_end(ap) 清理
  4. return ret
```

#### 不变量

无。本函数纯粹作为转发代理。

#### 依赖

- `vfscanf()` — 格式化输入核心引擎（见 `vfscanf.c`）
- `va_start` / `va_end` / `va_list` — C 标准可变参数宏

---

### 2. __isoc99_fscanf (weak_alias)

```c
weak_alias(fscanf, __isoc99_fscanf);
```

[Visibility]: Internal — 不对外导出（musl 内部兼容别名）

- **Intention**: 提供 C99 标准兼容的 `__isoc99_fscanf` 弱别名。与 `fscanf` 行为完全相同。
