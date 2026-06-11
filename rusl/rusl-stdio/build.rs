use std::process::Command;

fn main() {
    // -nostartfiles: 抑制 GCC 默认的 glibc crt*.o
    println!("cargo:rustc-link-arg=-nostartfiles");

    // 编译 snprintf C wrapper 为静态库
    let out_dir = std::env::var("OUT_DIR").unwrap();
    let src = "src/snprintf_wrapper.c";
    let obj = format!("{}/snprintf_wrapper.o", out_dir);
    let lib = format!("{}/libsnprintf_wrapper.a", out_dir);

    let status = Command::new("cc")
        .args(&["-c", "-o", &obj, src])
        .status()
        .expect("failed to compile snprintf_wrapper.c");
    if !status.success() {
        panic!("cc failed to compile snprintf_wrapper.c");
    }

    let status = Command::new("ar")
        .args(&["crus", &lib, &obj])
        .status()
        .expect("failed to create libsnprintf_wrapper.a");
    if !status.success() {
        panic!("ar failed to create static library");
    }

    println!("cargo:rustc-link-lib=static=snprintf_wrapper");
    println!("cargo:rustc-link-search=native={}", out_dir);
}
