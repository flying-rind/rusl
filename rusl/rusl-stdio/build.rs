
fn main() {
    // -nostartfiles: 抑制 GCC 默认的 glibc crt*.o
    println!("cargo:rustc-link-arg=-nostartfiles");
}
