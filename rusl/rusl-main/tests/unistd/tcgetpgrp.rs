use test_framework::test;

test!("test_tcgetpgrp_invalid_fd" {
    {
        // -1 is not a valid fd, so tcgetpgrp should return -1
        let ret = super::tcgetpgrp(-1);
        assert_eq!(ret, -1);
    }
});
