use test_framework::test;

test!("test_tcsetpgrp_invalid_fd" {
    {
        // -1 is not a valid fd, so tcsetpgrp should return -1
        let pgrp: i32 = 0;
        let ret = super::tcsetpgrp(-1, pgrp);
        assert_eq!(ret, -1);
    }
});
