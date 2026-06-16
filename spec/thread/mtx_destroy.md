# mtx_destroy.c 规约

> musl libc 的 C11 互斥锁销毁函数实现。对于私有互斥锁，操作为空（no-op）。

---

## 依赖图

```
mtx_destroy
  (无依赖 — 纯 no-op)
```

---

## 函数规约

### 1. mtx_destroy

```c
void mtx_destroy(mtx_t *mtx);
```

[Visibility]: User — 通过 `<threads.h>` 对外导出 (ISO C11 7.26.4.2)

#### Intent

销毁一个互斥锁对象。在 musl 中，由于 C11 互斥锁使用原子变量实现锁状态且无需释放内核资源，销毁操作为空。若要支持进程间共享互斥锁，则底层 `pthread_mutex_destroy` 需负责清理。

#### 前置条件

- `mtx != NULL`
- 互斥锁当前未被任何线程锁定
- 销毁后不可再使用该互斥锁

#### 后置条件

- 函数返回，无副作用

#### 系统算法

```
mtx_destroy(mtx):
  1. // 空操作 (private mutex no-op)
  2. return
```

#### 不变量

- 无全局或静态状态被修改

#### 依赖

- 无外部依赖
