//! 所有集成测试子模块

mod bcmp_test;
mod bcopy_test;
mod bzero_test;
mod explicit_bzero_test;
mod index_test;
mod memccpy_test;
mod memchr_test;
mod memcmp_test;
mod memcpy_test;
mod memmem_test;
mod memmove_test;
mod mempcpy_test;
mod memrchr_test;
mod memset_test;
mod rindex_test;
mod stpcpy_test;
mod stpncpy_test;
mod strcasecmp_test;
mod strcasestr_test;
mod strcat_test;
mod strchr_test;
mod strchrnul_test;
mod strcmp_test;
mod strcpy_test;
mod strcspn_test;
// mod strdup_test;
mod strerror_r_test;
mod strlcat_test;
mod strlcpy_test;
mod strlen_test;
mod strncasecmp_test;
mod strncat_test;
mod strncmp_test;
mod strncpy_test;
// mod strndup_test;
mod strnlen_test;
mod strpbrk_test;
mod strrchr_test;
mod strsep_test;
mod strsignal_test;
mod strspn_test;
mod strstr_test;
mod strtok_r_test;
mod strtok_test;
mod strverscmp_test;
mod swab_test;
mod wcpcpy_test;
mod wcpncpy_test;
mod wcscasecmp_l_test;
mod wcscasecmp_test;
mod wcscat_test;
mod wcschr_test;
mod wcscmp_test;
mod wcscpy_test;
mod wcscspn_test;
// mod wcsdup_test;
mod wcslen_test;
mod wcsncasecmp_l_test;
mod wcsncasecmp_test;
mod wcsncat_test;
mod wcsncmp_test;
mod wcsncpy_test;
mod wcsnlen_test;
mod wcspbrk_test;
mod wcsrchr_test;
mod wcsspn_test;
mod wcsstr_test;
mod wcstok_test;
mod wcswcs_test;
mod wmemchr_test;
mod wmemcmp_test;
mod wmemcpy_test;
mod wmemmove_test;
mod wmemset_test;

// 根据 rusl feature 选择导入源
#[cfg(feature = "rusl")]
mod imports {
    pub use rusl_string::*;
}
#[cfg(not(feature = "rusl"))]
mod imports {
    pub use rusl::api::string::*;
}