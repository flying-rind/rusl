# sem_destroy.c 规约

> musl libc 有名/匿名信号量销毁函数。

---

## 依赖图

```
sem_destroy
  (无内部依赖 — 直接返回 0)
```

---

## 函数规约

### 1. `sem_destroy`

```c
int sem_destroy(sem_t *sem);
```

[Visibility]: User — 通过 `<semaphore.h>` 对外导出

#### Intent

销毁一个匿名信号量（`sem_init` 创建）。在 musl 实现中，匿名信号量无内核资源需要释放，故此函数为无操作。

#### 前置条件

- `sem` 指向之前通过 `sem_init` 成功初始化的信号量

#### 后置条件

- 返回 `0`
- 信号量变为未初始化状态，后续使用行为未定义

#### 系统算法

```
sem_destroy(sem):
  直接返回 0（无操作）
```

#### 不变量

- 始终返回 0（musl 匿名信号量无需内核级资源释放）
- 调用者有责任确保无线程正在该信号量上阻塞
