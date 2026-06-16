# sem_open — Rust 接口归约

## 原始 C 接口
```c
sem_t *sem_open(const char *name, int flags, ...);
int sem_close(sem_t *sem);
```

---

## Rust 外部 ABI 接口

```rust
// 与 C ABI 兼容的底层导出函数
extern "C" fn sem_open(
    name: *const core::ffi::c_char,
    flags: core::ffi::c_int,
    ...
) -> *mut sem_t;

extern "C" fn sem_close(sem: *mut sem_t) -> core::ffi::c_int;
```

---

## 意图

打开或创建有名信号量。基于 `/dev/shm` 共享内存文件实现跨进程共享。包含两个对外导出函数：
- `sem_open`：打开/创建有名信号量，若 `flags` 含 `O_CREAT` 则可变参数提供 `mode` 和 `value`
- `sem_close`：关闭有名信号量，减少引用计数，计数归零时解除 mmap 映射

rusl 内部可使用 Rust 的全局状态管理（如 `Mutex<Vec<SemTabEntry>>`）替代 C 的原始静态数组 + 自旋锁。内部辅助函数均在模块内保持私有。

## 前置条件

- `name` 为非空指针，以 '/' 开头
- 若 `flags & O_CREAT`：可变参数包含 `mode_t mode` 和 `unsigned value`，且 `value <= SEM_VALUE_MAX`

## 后置条件

**sem_open:**
- Case 1 成功：返回指向 `sem_t` 的指针（mmap 映射的共享内存），`semtab` 中增加引用
- Case 2 失败：返回 `SEM_FAILED`（即 `null_mut`），`errno` 设置为相应错误码

**sem_close:**
- Case 1 引用计数 > 1：`refcnt--`，不解除映射，返回 `0`
- Case 2 引用计数降为 0：`semtab[i]` 被清空，`munmap(sem)` 解除映射，返回 `0`

## 不变量

- 所有共享同一名称的 `sem_open` 调用返回相同的 mmap 映射（通过 inode 去重）
- 操作期间禁用线程取消（`PTHREAD_CANCEL_DISABLE`）
- 通过临时文件 `link` + `unlink` 保证原子创建语义
- `semtab` 最大 `SEM_NSEMS_MAX`（256）个条目

## 算法

对外导出保持 ABI 兼容的 `extern "C"` 函数。内部使用 Rust 的安全抽象管理 semtab 和文件操作。

### 内部状态设计（Safe Rust，模块私有）

```rust
use crate::sync::Mutex;

const SEM_NSEMS_MAX: usize = 256;

// semtab 条目（替代 C 的原始结构体数组）
struct SemTabEntry {
    ino: u64,
    sem: *mut sem_t,      // mmap 映射的信号量指针
    refcnt: usize,         // 引用计数
}

// 全局 semtab（使用 Rust Mutex 替代 C 的自旋锁 LOCK/UNLOCK）
struct SemTab {
    entries: [Option<SemTabEntry>; SEM_NSEMS_MAX],
}

static SEMTAB: Mutex<SemTab> = Mutex::new(SemTab::new());
```

### extern "C" sem_open 内部流程（Safe Rust）

```
sem_open(name, flags, ...):
  1. 将 name 映射为 /dev/shm/ 路径（__shm_mapname）
     失败 → 返回 SEM_FAILED
  2. 获取 SEMTAB 锁，分配空闲槽位（哨兵标记）
  3. 释放锁
  4. 若 flags == (O_CREAT|O_EXCL) 且文件已存在 → errno = EEXIST → fail
  5. 循环尝试：
     a. 若非 O_CREAT|O_EXCL 模式：尝试直接打开已有文件 → mmap
     b. 若非 O_CREAT → 失败
     c. 解析可变参数 mode 和 value
     d. 验证 value <= SEM_VALUE_MAX
     e. 创建临时文件，写入初始化数据
     f. mmap 临时文件
     g. link(临时名, 目标名) → 若 EEXIST 且非排他模式则重试
     h. unlink(临时名)
  6. 获取 SEMTAB 锁，通过 inode 去重
     - 若已有相同 inode 的映射 → munmap 当前映射，复用已有映射
     - 否则将当前映射登记到预留槽位
  7. refcnt++, 释放锁 → 返回 map

fail:
  恢复取消状态 → 获取锁 → 释放预留槽位 → 释放锁 → 返回 SEM_FAILED
```

### extern "C" sem_close 内部流程

```
sem_close(sem):
  1. 获取 SEMTAB 锁
  2. 查找 semtab[i].sem == sem
  3. refcnt--
  4. 若 refcnt != 0 → 释放锁，返回 0
  5. 标记 semtab[i] 为空闲，释放锁
  6. munmap(sem)
  7. 返回 0
```

Note: 原始 C 的 `sem_open.c` 中有大量内部静态函数（如 semtab 管理、临时文件创建等），这些在 rusl 中均为模块私有函数，不对外暴露，可使用 Safe Rust 自由重新设计。

---

## Rust 安全包装（模块内部，不对外暴露）

```rust
// 模块私有函数（供 sem_open / sem_close 的 extern "C" 实现内部调用）
fn map_name_to_shm_path(name: &core::ffi::CStr) -> Result<PathBuf, ()>;
fn create_sem_file(path: &Path, value: u32) -> Result<(FileDescriptor, u64), Errno>;
fn open_existing_sem(path: &Path) -> Result<(*mut sem_t, u64), Errno>;
```

---

/* Rely */
[RELY]
Predefined Structures/Functions:
  __shm_mapname()                                    // 依赖1: 将 posix 名称映射为 /dev/shm 文件路径
  sem_init()                                         // 依赖2: 初始化信号量结构
  pthread_setcancelstate()                           // 依赖3: 取消状态控制
  mmap() / munmap()                                  // 依赖4: 共享内存映射/解除
  access() / open() / close() / write() / link() / unlink() // 依赖5: 文件系统操作
  fstat()                                            // 依赖6: 获取文件 inode 信息
  clock_gettime()                                    // 依赖7: 获取纳秒时间戳（生成唯一临时文件名）
  crate::stdio::snprintf()                           // 依赖8: 格式化路径字符串
  errno                                              // 依赖9: 错误码
  SEM_NSEMS_MAX (256)                                // 依赖10: semtab 最大条目数
Predefined Macros/Traits:
  O_CREAT / O_EXCL / O_RDWR / O_CLOEXEC             // 依赖11: open 标志组合
  crate::sync::Mutex                                 // 依赖12: Rust 互斥锁（替代 C 自旋锁）

[GUARANTEE]
Exported Interface:
  extern "C" fn sem_open(name: *const core::ffi::c_char, flags: core::ffi::c_int, ...) -> *mut sem_t;
  extern "C" fn sem_close(sem: *mut sem_t) -> core::ffi::c_int;
Internal Interface:
  (全部为模块私有函数，不对外暴露)
  模块内部: semtab 管理、临时文件创建、inode 去重等均为私有辅助函数
