# atoll Rust 接口

## 复杂度分级: Level 2

---

## Rust 接口

```rust
#[no_mangle]
pub unsafe extern "C" fn atoll(s: *const c_char) -> i64;
```

### 前置/后置条件

同 atoi，返回类型为 i64。负向累加策略解析十进制字符串。