use test_framework::test;

test!("test_getegid_basic" {
    {
        let egid = super::getegid();
        // getegid always succeeds; egid is a u32
        let _ = egid;
    }
});

test!("test_getegid_idempotent" {
    {
        let egid1 = super::getegid();
        let egid2 = super::getegid();
        assert_eq!(egid1, egid2);
    }
});
