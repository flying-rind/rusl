/// 构建脚本
///
/// 当启用 `c-test` feature 时,链接 musl libc 预编译的 `libc.a` 及启动文件,
/// 通过 musl 原生 CRT (`Scrt1.o` → `__libc_start_main` → `main`)
/// 自动完成 TLS/FS/stdio 等全部初始化。
///
/// 非 c-test 模式: `-nostartfiles`,由 Rust 侧 `_start` 直接进入测试,
/// 编译 regex FFI stubs 和 stdio wrappers 供内部实现使用。
///
/// 注意: 启用 c-test 时,rusl 内部模块不编译,避免符号冲突。

fn main() {
    // -nostartfiles: 抑制 GCC 默认的 glibc crt*.o (两种模式都需要)
    println!("cargo:rustc-link-arg=-nostartfiles");

    #[cfg(not(feature = "rusl"))]
    link_musl_libc();
}

/// 链接 musl libc — 测试 musl libc 的对外导出符号
#[cfg(not(feature = "rusl"))]
fn link_musl_libc() {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let musl_root = std::path::Path::new(&manifest_dir)
        .parent()
        .and_then(|p| p.parent())
        .expect("无法定位 musl 源码根目录");

    let lib_dir = musl_root.join("musl-config/origin/lib");

    // musl 原生启动文件: _start → __libc_start_main → main
    // Scrt1.o 用于 PIE (Rust 默认), crti.o/crtn.o 提供 .init/.fini 段头尾
    println!("cargo:rustc-link-arg={}", lib_dir.join("Scrt1.o").display());
    println!("cargo:rustc-link-arg={}", lib_dir.join("crti.o").display());

    println!("cargo:rustc-link-search=native={}", lib_dir.display());
    println!("cargo:rustc-link-lib=static=c");

    println!("cargo:rustc-link-arg={}", lib_dir.join("crtn.o").display());

    println!("cargo:rustc-link-arg=-lgcc_eh");
    println!("cargo:rustc-link-arg=-lgcc");

    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed={}", lib_dir.join("libc.a").display());
}

