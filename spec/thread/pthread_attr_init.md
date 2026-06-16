# pthread_attr_init.c 规约

> musl libc 线程属性对象初始化函数。将属性对象清零，然后使用模块默认值填充栈大小和守护页大小。在持有 PTC 锁的保护下读取全局默认值。

---

## 依赖图

```
pthread_attr_init
  ├─> __acquire_ptc()       (PTC 锁获取, pthread_impl.h)
  ├─> __release_ptc()       (PTC 锁释放, pthread_impl.h)
  ├─> __default_stacksize   (全局变量, default_attr.c)
  └─> __default_guardsize   (全局变量, default_attr.c)
```

---

## 函数规约

### 1. pthread_attr_init

```c
int pthread_attr_init(pthread_attr_t *a);
```

[Visibility]: User — 通过 `<pthread.h>` 对外导出 (POSIX)

#### Intent

初始化线程属性对象，将默认值（零初始化 + 模块默认栈/守护页大小）写入属性对象。PTC 锁用于保证读取全局默认值时的线程安全性。

#### 前置条件

- `a != NULL`，指向一个未初始化的 `pthread_attr_t` 内存区域

#### 后置条件

- Case 1 始终成功：返回 `0`
- 操作步骤：
  1. `*a` 全部字段清零（`(pthread_attr_t){0}`）
  2. `__acquire_ptc()` — 获取 PTC 锁
  3. `a->_a_stacksize = __default_stacksize` — 填入模块默认栈大小
  4. `a->_a_guardsize = __default_guardsize` — 填入模块默认守护页大小
  5. `__release_ptc()` — 释放 PTC 锁

#### 不变量

- 函数执行期间 PTC 锁被持有，保证 `__default_stacksize` / `__default_guardsize` 的读取一致性
- `pthread_attr_t` 结构体其余成员保持零值（detach=0/joinable, sched=0/inherit, policy=0, prio=0, stackaddr=0）

#### 系统算法

```
pthread_attr_init(a):
  1. 将 a 的整个内存区域以值 0 填充 (复合字面量赋值)
  2. __acquire_ptc() — 锁保护，确保原子读取全局默认值
  3. a->_a_stacksize = __default_stacksize
  4. a->_a_guardsize = __default_guardsize
  5. __release_ptc()
  6. return 0
```

#### 依赖

- `__acquire_ptc()` — PTC 锁获取（定义于 thread 模块内部）
- `__release_ptc()` — PTC 锁释放（定义于 thread 模块内部）
- `__default_stacksize` — 全局变量（定义于 `default_attr.c`）
- `__default_guardsize` — 全局变量（定义于 `default_attr.c`）
- `pthread_impl.h` — 内部头文件，定义 `_a_stacksize` / `_a_guardsize` 宏及函数声明
