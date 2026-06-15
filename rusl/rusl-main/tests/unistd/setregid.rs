use test_framework::test;

test!("test_setregid_no_change" {
    {
        // -1u32 means "don't change" for both rgid and egid
        let ret = super::setregid(!0u32, !0u32);
        assert_eq!(ret, 0);
    }
});

test!("test_setregid_same_ids" {
    {
        let gid = super::getgid();
        let egid = super::getegid();
        let ret = super::setregid(gid, egid);
        assert_eq!(ret, 0);
    }
});
