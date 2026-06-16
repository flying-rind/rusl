# pthread_attr_setdetachstate.c 规约

> musl libc 线程属性设置函数（detachstate）。设置线程创建时的分离状态，仅接受 `PTHREAD_CREATE_JOINABLE` (0) 或 `PTHREAD_CREATE_DETACHED` (1)。

---

## 依赖图

```
pthread_attr_setdetachstate
  (无函数调用依赖)
```

---

## 函数规约

### 1. pthread_attr_setdetachstate

```c
int pthread_attr_setdetachstate(pthread_attr_t *a, int state);
```

[Visibility]: User — 通过 `<pthread.h>` 对外导出 (POSIX)

#### Intent

设置线程属性中的分离状态。`PTHREAD_CREATE_JOINABLE` 表示线程可被等待，`PTHREAD_CREATE_DETACHED` 表示线程退出后自动释放资源。

#### 前置条件

- `a != NULL`，指向已初始化的 `pthread_attr_t`

#### 后置条件

- Case 1 有效值（`state <= 1U`）：
  - `a->_a_detach = state`
  - 返回 `0`
- Case 2 无效值（`state > 1U`）：
  - 返回 `EINVAL`
  - 属性对象不被修改

#### 不变量

- 该函数不访问全局状态
- `state` 的有效取值范围为 {0, 1}（分别对应 `PTHREAD_CREATE_JOINABLE`, `PTHREAD_CREATE_DETACHED`）

#### 系统算法

```
pthread_attr_setdetachstate(a, state):
  1. if (state > 1U) return EINVAL
  2. a->_a_detach = state
  3. return 0
```

#### 依赖

- `pthread_impl.h` — 内部头文件，定义 `_a_detach` 宏
- `EINVAL` — 宏，定义于 `<errno.h>`
