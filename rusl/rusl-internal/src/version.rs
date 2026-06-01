// Version string.
//
// Corresponds to musl's src/internal/version.c
//
// In musl, version.h is generated from VERSION file + git describe.
// We embed the version directly here.
// Must be `const char[]` in C terms (array, not pointer).

/// libc version string, exposed as a C symbol.
/// C programs can access: extern const char __libc_version[];
/// This is a byte array (inline), NOT a pointer.
#[no_mangle]
#[link_section = ".rodata"]
pub static __libc_version: [u8; 6] = *b"1.2.6\0";

// Convenience for Rust code that wants a string slice.
#[allow(dead_code)]
pub fn version() -> &'static str {
    "1.2.6"
}