use test_framework::test;

test!("test_setresgid_no_change" {
    {
        // -1u32 means "don't change" for rgid, egid, and sgid
        let ret = super::setresgid(!0u32, !0u32, !0u32);
        assert_eq!(ret, 0);
    }
});
