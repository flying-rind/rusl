use test_framework::test;

test!("test_getgid_basic" {
    {
        let gid = super::getgid();
        // getgid always succeeds; gid is a u32
        let _ = gid;
    }
});

test!("test_getgid_idempotent" {
    {
        let gid1 = super::getgid();
        let gid2 = super::getgid();
        assert_eq!(gid1, gid2);
    }
});
