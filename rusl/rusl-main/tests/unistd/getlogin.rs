use test_framework::test;

test!("test_getlogin_basic" {
    {
        let result = super::getlogin();
        // getlogin() returns a pointer to the LOGNAME environment variable
        // or NULL if LOGNAME is not set.
        // Just verify the function doesn't crash.
        if !result.is_null() {
            unsafe {
                // Should be a valid null-terminated string
                assert_ne!(*result, 0);
            }
        }
    }
});
