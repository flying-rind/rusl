# mtx_destroy — Rust 接口归约

## 原始 C 接口
```c
void mtx_destroy(mtx_t *mtx);
```

[Visibility]: User — 通过 `<threads.h>` 对外导出 (ISO C11 7.26.4.2)

---

## Rust 外部 ABI 接口

```rust
// 与 C ABI 兼容的底层导出函数
extern "C" fn mtx_destroy(mtx: *mut mtx_t);
```

---

## 意图
销毁一个互斥锁对象。在 musl 中，由于 C11 互斥锁使用原子变量实现锁状态且无需释放内核资源，销毁操作为空。

## 前置条件
- `mtx` 为非空指针（`!mtx.is_null()`）
- 互斥锁当前未被任何线程锁定
- 销毁后不可再使用该互斥锁

## 后置条件
- 函数返回，无副作用

## 不变量
- 无全局或静态状态被修改

## 算法
```rust
extern "C" fn mtx_destroy(_mtx: *mut mtx_t) {
    // 私有互斥锁销毁是空操作（no-op）
    // 对于进程间共享或健壮互斥锁，需调用底层 pthread_mutex_destroy
}
```

---

/* Rely */
[RELY]
Predefined Structures/Functions:
  (无外部依赖)
Predefined Macros/Types:
  mtx_t (= pthread_mutex_t)            // C11 互斥锁类型

[GUARANTEE]
Exported Interface:
  extern "C" fn mtx_destroy(mtx: *mut mtx_t);
                                        // 本模块保证对外提供与 C ABI 兼容的 mtx_destroy 符号
Internal Interface:
  (无内部导出)
