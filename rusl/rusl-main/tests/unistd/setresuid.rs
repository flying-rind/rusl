use test_framework::test;

test!("test_setresuid_no_change" {
    {
        // -1u32 means "don't change" for ruid, euid, and suid
        let ret = super::setresuid(!0u32, !0u32, !0u32);
        assert_eq!(ret, 0);
    }
});
