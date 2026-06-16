# pthread_attr_setguardsize.c 规约

> musl libc 线程属性设置函数（guardsize）。设置线程栈守护页大小，对输入值进行溢出安全性检查。

---

## 依赖图

```
pthread_attr_setguardsize
  (无函数调用依赖)
```

---

## 函数规约

### 1. pthread_attr_setguardsize

```c
int pthread_attr_setguardsize(pthread_attr_t *a, size_t size);
```

[Visibility]: User — 通过 `<pthread.h>` 对外导出 (POSIX)

#### Intent

设置线程属性中的守护页大小。守护页位于线程栈末尾，配置为不可访问，用于检测栈溢出。校验规则：`size` 不允许超过 `SIZE_MAX/8`，防止后续计算溢出。

#### 前置条件

- `a != NULL`，指向已初始化的 `pthread_attr_t`

#### 后置条件

- Case 1 有效值（`size <= SIZE_MAX/8`）：
  - `a->_a_guardsize = size`
  - 返回 `0`
- Case 2 超出允许范围（`size > SIZE_MAX/8`）：
  - 返回 `EINVAL`
  - 属性对象不被修改

#### 不变量

- 该函数不访问全局状态
- 上限 `SIZE_MAX/8` 确保在后续栈大小 + 守护页大小计算时不会溢出

#### 系统算法

```
pthread_attr_setguardsize(a, size):
  1. if (size > SIZE_MAX/8) return EINVAL
  2. a->_a_guardsize = size
  3. return 0
```

#### 依赖

- `pthread_impl.h` — 内部头文件，定义 `_a_guardsize` 宏
- `EINVAL` — 宏，定义于 `<errno.h>`
- `SIZE_MAX` — 宏，定义于 `<limits.h>` 或 `<stdint.h>`
