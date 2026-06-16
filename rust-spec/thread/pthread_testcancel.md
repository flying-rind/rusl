# pthread_testcancel — Rust 接口归约

## 原始 C 接口
```c
// 对外导出（POSIX），是 __pthread_testcancel 的弱别名
void pthread_testcancel(void);

// 内部（hidden），pthread_testcancel 的主实现
void __pthread_testcancel(void);

// 内部 static 占位空函数
static void dummy(void);
```

---

## Rust 外部 ABI 接口

```rust
// 与 C ABI 兼容的底层导出函数
// musl 中 pthread_testcancel 是 __pthread_testcancel 的弱别名
// Rust 中必须同时导出两者

extern "C" fn pthread_testcancel();

extern "C" fn __pthread_testcancel();
```

---

## 意图
在调用点显式检查是否有挂起的取消请求。如果当前线程已被其他线程通过 `pthread_cancel` 请求取消，且取消未被禁用（`canceldisable == PTHREAD_CANCEL_ENABLE`），则在此处执行取消并终止线程。

## 前置条件
- 调用线程的取消状态为启用（`canceldisable != PTHREAD_CANCEL_DISABLE`）

## 后置条件
- Case 1（存在挂起的取消请求）：调用 `__cancel()` 执行取消，线程终止（不返回）
- Case 2（无挂起的取消请求或取消被禁用）：立即返回，线程继续正常执行

## 不变量
- 取消检查是幂等的：多次调用结果相同（除非状态在调用间被修改）

## 算法

```rust
// __pthread_testcancel — 主入口，委托给 __testcancel
pub extern "C" fn __pthread_testcancel() {
    // 检查当前线程的 cancel 标志
    // 若 cancel 已设置且 canceldisable 为启用状态，执行取消
    __testcancel();  // 定义在 pthread_cancel 模块中
}

// pthread_testcancel — POSIX 用户接口，等同于 __pthread_testcancel
pub extern "C" fn pthread_testcancel() {
    __pthread_testcancel();
}
```

对 C 调用者：
1. `extern "C" fn pthread_testcancel()` 无参数
2. 内部调用 `__testcancel()`：检查 `self.cancel && !self.canceldisable`
3. 若条件满足，调用 `__cancel()` 终止线程（不返回）

---

## Rust 安全包装（模块内部，不对外暴露）

```rust
// 安全的 Rust 封装（供内部使用）
// 在线程上下文中检查取消状态
pub(crate) fn testcancel() {
    // 通过线程本地存储获取当前线程的取消状态
    // 若 cancel && canceldisable == ENABLE，则触发取消退出
    let current = current_thread();  // 从 TLS/线程指针获取
    if current.cancel() && !current.cancel_disabled() {
        current.do_cancel();  // 执行取消，可能不返回
    }
}
```

---

/* Rely */
[RELY]
Predefined Types/Functions:
  __testcancel()                    // 依赖1: 取消检查核心（定义在 pthread_cancel 模块中）
  __pthread_self()                  // 依赖2: 获取当前线程 struct pthread *（通过 TLS）
  __cancel()                        // 依赖3: 执行实际的取消退出（定义在 pthread_cancel 模块中）
Predefined Macros/Constants:
  PTHREAD_CANCEL_ENABLE (0)        // 依赖4: 取消启用常量
  PTHREAD_CANCEL_DISABLE (1)       // 依赖5: 取消禁用常量

[GUARANTEE]
Exported Interface:
  extern "C" fn pthread_testcancel();
  extern "C" fn __pthread_testcancel();
                                    // 本模块保证对外提供与 C ABI 兼容的 pthread_testcancel 和 __pthread_testcancel 符号
Internal Interface:
  pub(crate) fn testcancel();
                                    // 安全包装，供 crate 内部使用
