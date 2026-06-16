# cnd_destroy.c 规约

> musl libc 的 C11 条件变量销毁函数实现。对于私有条件变量，操作为空（no-op）。

---

## 依赖图

```
cnd_destroy
  (无依赖 — 纯 no-op)
```

---

## 函数规约

### 1. cnd_destroy

```c
void cnd_destroy(cnd_t *c);
```

[Visibility]: User — 通过 `<threads.h>` 对外导出 (ISO C11 7.26.3.2)

#### Intent

销毁一个条件变量对象。在 musl 中，私有条件变量的销毁是空操作，因为条件变量不包含需要释放的内核资源。

#### 前置条件

- `c != NULL`
- 没有线程正在该条件变量上等待
- 销毁后不可再使用 `c`

#### 后置条件

- 函数返回（空操作），无副作用
- 对于进程间共享的条件变量，应由 `pthread_cond_destroy` 处理（此处未使用）

#### 系统算法

```
cnd_destroy(c):
  1. // For private cv this is a no-op
  2. return
```

#### 不变量

- 无全局或静态状态被修改

#### 依赖

- 无外部依赖
