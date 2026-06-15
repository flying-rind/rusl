use test_framework::test;

test!("test_ttyname_invalid_fd" {
    {
        // -1 is not a valid fd, so ttyname should return NULL
        let result = super::ttyname(-1);
        assert!(result.is_null());
    }
});
