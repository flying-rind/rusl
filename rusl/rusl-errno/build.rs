fn main() {
    // -nostartfiles: 抑制 GCC 默认的 glibc crt*.o (两种模式都需要)
    println!("cargo:rustc-link-arg=-nostartfiles");
}
