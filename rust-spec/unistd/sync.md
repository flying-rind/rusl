# sync — Rust 接口归约

## 原始 C 接口
```c
void sync(void);
```

---

## Rust 外部 ABI 接口

```rust
// 与 C ABI 兼容的底层导出函数
extern "C" fn sync();
```

---

## 意图
将所有已修改的文件系统缓冲区和元数据写入磁盘。调用不等待 I/O 完成即返回。通常由系统管理员在关机前调用以确保数据完整性。

## 前置条件
- 无

## 后置条件
- 所有脏缓冲区已提交到磁盘 I/O 队列（但可能尚未物理写入），函数返回
- 无返回值

## 不变量
无。

## 算法
原 C 实现：`__syscall(SYS_sync)`。

```rust
#[inline]
fn sys_sync() {
    unsafe {
        syscall!(SYS_sync);
    }
}
```

---

/* Rely */
[RELY]
Predefined Structures/Functions:
  SYS_sync                                    // 依赖1: Linux 系统调用编号
Predefined Macros/Crates:
  syscall! 宏                                 // 依赖2: 系统调用入口

[GUARANTEE]
Exported Interface:
  extern "C" fn sync();
                                   // 本模块保证对外提供与 C ABI 兼容的 sync 符号
Internal Interface:
  (无额外内部接口)
